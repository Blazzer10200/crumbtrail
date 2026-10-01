// "What changed" snapshots: each full-drive scan saves folder sizes (no file
// names, no contents) so the next scan can show which folders grew or shrank.
// Stored as JSON in %LOCALAPPDATA%\Crumbtrail\snapshots, newest 12 per drive.

use crate::space::SpaceScan;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const KEEP: usize = 12;
const MIN_FOLDER: u64 = 50 * 1024 * 1024;
const MIN_DELTA: u64 = 50 * 1024 * 1024;
const MAX_DEPTH: usize = 6;
// Re-scanning within this window replaces the previous snapshot instead of piling up.
const DEDUPE_SECS: i64 = 3600;

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub drive: String,
    pub taken_at: i64,
    pub used_bytes: u64,
    pub folders: HashMap<String, u64>,
}

#[derive(Serialize, Clone)]
pub struct SnapMeta {
    pub id: String,
    pub taken_at: i64,
    pub used_bytes: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct DiffRow {
    pub path: String,
    pub name: String,
    pub now: u64,
    pub then: u64,
    pub delta: i64,
    pub is_new: bool,
}

#[derive(Serialize)]
pub struct Diff {
    pub net: i64,
    pub grew: Vec<DiffRow>,
    pub shrank: Vec<DiffRow>,
}

pub fn dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Crumbtrail")
        .join("snapshots")
}

// Depth below the drive root: "C:\" = 0, "C:\Users" = 1.
fn depth(p: &Path) -> usize {
    p.components().count().saturating_sub(2)
}

pub fn from_scan(scan: &SpaceScan, drive: &str, used_bytes: u64, taken_at: i64) -> Snapshot {
    let folders = scan
        .dirs
        .iter()
        .filter(|(p, &b)| b > MIN_FOLDER && depth(p) <= MAX_DEPTH)
        .map(|(p, &b)| (p.display().to_string(), b))
        .collect();
    Snapshot {
        drive: drive.to_string(),
        taken_at,
        used_bytes,
        folders,
    }
}

fn file_id(drive: &str, taken_at: i64) -> String {
    format!("{}-{}", drive.trim_end_matches(':'), taken_at)
}

// "C-1727712000" only — ids come from the UI, so never let one walk out of the folder.
fn valid_id(id: &str) -> bool {
    let mut it = id.splitn(2, '-');
    matches!(
        (it.next(), it.next()),
        (Some(d), Some(t)) if d.len() == 1
            && d.bytes().all(|b| b.is_ascii_uppercase())
            && !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_digit())
    )
}

pub fn list_in(base: &Path, drive: &str) -> Vec<SnapMeta> {
    let prefix = format!("{}-", drive.trim_end_matches(':'));
    let Ok(rd) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out: Vec<SnapMeta> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let id = name.strip_suffix(".json")?.to_string();
            if !id.starts_with(&prefix) || !valid_id(&id) {
                return None;
            }
            let snap = load_in(base, &id).ok()?;
            Some(SnapMeta {
                id,
                taken_at: snap.taken_at,
                used_bytes: snap.used_bytes,
            })
        })
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.taken_at));
    out
}

pub fn load_in(base: &Path, id: &str) -> Result<Snapshot, String> {
    if !valid_id(id) {
        return Err(format!("bad snapshot id {id}"));
    }
    let text = std::fs::read_to_string(base.join(format!("{id}.json")))
        .map_err(|e| format!("couldn't read snapshot {id}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("snapshot {id} is damaged: {e}"))
}

/// Save, drop a same-drive snapshot taken within the last hour, keep the newest 12.
pub fn save_in(base: &Path, snap: &Snapshot) -> Result<SnapMeta, String> {
    std::fs::create_dir_all(base)
        .map_err(|e| format!("couldn't create {}: {e}", base.display()))?;
    let id = file_id(&snap.drive, snap.taken_at);
    let text = serde_json::to_string(snap).map_err(|e| e.to_string())?;
    let tmp = base.join(format!("{id}.json.tmp"));
    std::fs::write(&tmp, text).map_err(|e| format!("couldn't write snapshot: {e}"))?;
    std::fs::rename(&tmp, base.join(format!("{id}.json")))
        .map_err(|e| format!("couldn't write snapshot: {e}"))?;

    let mut kept = 0;
    for m in list_in(base, &snap.drive) {
        let recent_dupe = m.id != id && snap.taken_at - m.taken_at < DEDUPE_SECS;
        if recent_dupe || kept >= KEEP {
            let _ = std::fs::remove_file(base.join(format!("{}.json", m.id)));
        } else {
            kept += 1;
        }
    }
    Ok(SnapMeta {
        id,
        taken_at: snap.taken_at,
        used_bytes: snap.used_bytes,
    })
}

fn lc(s: &str) -> String {
    s.to_lowercase()
}

fn under(path_lc: &str, root_lc: &str) -> bool {
    let r = root_lc.trim_end_matches('\\');
    path_lc.len() > r.len() + 1 && path_lc.starts_with(r) && path_lc.as_bytes()[r.len()] == b'\\'
}

fn parent_lc(p: &str) -> Option<&str> {
    let t = p.trim_end_matches('\\');
    t.rfind('\\')
        .map(|i| if i <= 2 { &t[..=i] } else { &t[..i] })
}

/// Compare the current scan (folders under `root`) with an older snapshot.
/// Only the most specific folder that explains a change is listed: a parent whose
/// change is >=80% explained by one child is dropped. `net` is the drive's used-bytes
/// change for a drive scan, or the scanned folder's own change for a folder scan.
pub fn diff(scan: &SpaceScan, old: &Snapshot) -> Diff {
    let root = scan.root.display().to_string();
    let root_lc = lc(&root);
    let is_drive = root.trim_end_matches('\\').len() <= 2;

    let now: HashMap<String, (String, u64)> = scan
        .dirs
        .iter()
        .filter(|(p, _)| depth(p) <= MAX_DEPTH)
        .map(|(p, &b)| {
            let s = p.display().to_string();
            (lc(&s), (s, b))
        })
        .collect();
    let then: HashMap<String, (String, u64)> = old
        .folders
        .iter()
        .map(|(p, &b)| (lc(p), (p.clone(), b)))
        .collect();

    let mut keys: Vec<&String> = then
        .keys()
        .chain(
            now.iter()
                .filter(|(_, (_, b))| *b > MIN_FOLDER)
                .map(|(k, _)| k),
        )
        .filter(|k| under(k, &root_lc))
        .collect();
    keys.sort();
    keys.dedup();

    let mut rows: Vec<(String, DiffRow)> = keys
        .into_iter()
        .filter_map(|k| {
            let n = now.get(k).map(|x| x.1).unwrap_or(0);
            let t = then.get(k).map(|x| x.1).unwrap_or(0);
            let delta = n as i64 - t as i64;
            if delta.unsigned_abs() <= MIN_DELTA {
                return None;
            }
            let path = now.get(k).or_else(|| then.get(k)).map(|x| x.0.clone())?;
            let name = path.rsplit('\\').next().unwrap_or(&path).to_string();
            Some((
                k.clone(),
                DiffRow {
                    path,
                    name,
                    now: n,
                    then: t,
                    delta,
                    is_new: !then.contains_key(k),
                },
            ))
        })
        .collect();

    // Biggest same-direction child change per parent.
    let mut best_child: HashMap<String, (i64, i64)> = HashMap::new(); // (max growth, max shrink)
    for (k, r) in &rows {
        if let Some(p) = parent_lc(k) {
            let e = best_child.entry(p.to_string()).or_insert((0, 0));
            if r.delta > 0 {
                e.0 = e.0.max(r.delta);
            } else {
                e.1 = e.1.min(r.delta);
            }
        }
    }
    rows.retain(|(k, r)| {
        let (up, down) = best_child.get(k).copied().unwrap_or((0, 0));
        let child = if r.delta > 0 { up } else { down };
        // child / parent >= 0.8, kept in integers
        !(child != 0 && child.unsigned_abs() * 5 >= r.delta.unsigned_abs() * 4)
    });

    let mut grew: Vec<DiffRow> = rows
        .iter()
        .filter(|(_, r)| r.delta > 0)
        .map(|(_, r)| r.clone())
        .collect();
    let mut shrank: Vec<DiffRow> = rows
        .into_iter()
        .filter(|(_, r)| r.delta < 0)
        .map(|(_, r)| r)
        .collect();
    grew.sort_by_key(|r| std::cmp::Reverse(r.delta));
    shrank.sort_by_key(|r| r.delta);
    grew.truncate(40);
    shrank.truncate(20);

    let net = if is_drive {
        scan.used_bytes.unwrap_or(0) as i64 - old.used_bytes as i64
    } else {
        let n = now
            .get(root_lc.trim_end_matches('\\'))
            .map(|x| x.1)
            .unwrap_or(0);
        let t = then
            .get(root_lc.trim_end_matches('\\'))
            .map(|x| x.1)
            .unwrap_or(0);
        n as i64 - t as i64
    };
    Diff { net, grew, shrank }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const MB: u64 = 1024 * 1024;

    fn scan_of(root: &str, dirs: &[(&str, u64)], used: u64) -> SpaceScan {
        SpaceScan {
            root: PathBuf::from(root),
            dirs: dirs.iter().map(|(p, b)| (PathBuf::from(p), *b)).collect(),
            biggest: Vec::new(),
            files: 0,
            bytes: 0,
            types: [0; 7],
            type_top: Vec::new(),
            unreadable: Vec::new(),
            unreadable_count: 0,
            used_bytes: Some(used),
        }
    }

    fn snap(dirs: &[(&str, u64)], used: u64) -> Snapshot {
        Snapshot {
            drive: "C:".into(),
            taken_at: 1,
            used_bytes: used,
            folders: dirs.iter().map(|(p, b)| (p.to_string(), *b)).collect(),
        }
    }

    #[test]
    fn diff_lists_most_specific_folder_and_uses_drive_delta() {
        let old = snap(
            &[
                ("C:\\Users", 10_000 * MB),
                ("C:\\Users\\Alex", 9_000 * MB),
                ("C:\\Users\\Alex\\Videos", 2_000 * MB),
                ("C:\\Users\\Alex\\Videos\\Captures", 1_000 * MB),
                ("C:\\Temp", 400 * MB),
            ],
            100_000 * MB,
        );
        // Captures +7000, a new game folder +5000, Temp -300, tiny wobble ignored.
        let scan = scan_of(
            "C:\\",
            &[
                ("C:\\Users", 17_000 * MB),
                ("c:\\users\\alex", 16_000 * MB),
                ("C:\\Users\\Alex\\Videos", 9_000 * MB),
                ("C:\\Users\\Alex\\Videos\\Captures", 8_000 * MB),
                ("C:\\Games", 5_000 * MB),
                ("C:\\Games\\Hades II", 5_000 * MB),
                ("C:\\Temp", 100 * MB),
            ],
            111_500 * MB,
        );
        let d = diff(&scan, &old);
        let grew: Vec<&str> = d.grew.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(grew, ["Captures", "Hades II"]);
        assert!(d.grew[1].is_new);
        assert!(!d.grew[0].is_new);
        assert_eq!(d.shrank.len(), 1);
        assert_eq!(d.shrank[0].delta, -(300 * MB as i64));
        assert_eq!(d.net, 11_500 * MB as i64);
    }

    #[test]
    fn parent_stays_when_no_single_child_explains_it() {
        let old = snap(&[("C:\\Data", 1_000 * MB)], 0);
        let scan = scan_of(
            "C:\\",
            &[
                ("C:\\Data", 2_000 * MB),
                ("C:\\Data\\A", 520 * MB),
                ("C:\\Data\\B", 480 * MB),
            ],
            0,
        );
        let d = diff(&scan, &old);
        let names: HashSet<&str> = d.grew.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains("Data"));
    }

    #[test]
    fn save_dedupes_rapid_rescans_and_keeps_twelve() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..15 {
            let mut s = snap(&[], i);
            s.taken_at = 1_000_000 + i as i64 * 86_400;
            save_in(tmp.path(), &s).unwrap();
        }
        assert_eq!(list_in(tmp.path(), "C:").len(), KEEP);

        let mut s = snap(&[], 99);
        s.taken_at = 1_000_000 + 14 * 86_400 + 60; // a minute after the newest
        save_in(tmp.path(), &s).unwrap();
        let l = list_in(tmp.path(), "C:");
        assert_eq!(l.len(), KEEP);
        assert_eq!(l[0].used_bytes, 99);
        assert!(l[1].taken_at <= s.taken_at - 86_400);
    }

    #[test]
    fn ids_cannot_escape_the_folder() {
        assert!(valid_id("C-1727712000"));
        assert!(!valid_id("..\\x-1"));
        assert!(!valid_id("C-12a"));
        assert!(load_in(Path::new("."), "../../etc").is_err());
    }
}
