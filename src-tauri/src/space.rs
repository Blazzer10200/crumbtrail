use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// How many of the largest individual files to keep during a scan.
const TOP_FILES: usize = 200;

#[derive(Default)]
pub struct SpaceState(pub Mutex<Option<SpaceScan>>);

pub struct SpaceScan {
    pub root: PathBuf,
    pub dirs: HashMap<PathBuf, u64>,
    pub biggest: Vec<FolderEntry>,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Serialize, Clone)]
pub struct DriveInfo {
    pub letter: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Serialize, Clone)]
pub struct FolderEntry {
    pub path: String,
    pub name: String,
    pub bytes: u64,
}

pub fn list_drives() -> Vec<DriveInfo> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
    };
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root: Vec<u16> = format!("{}:\\", letter).encode_utf16().chain([0]).collect();
        // 3 = DRIVE_FIXED — local disks only, no USB/network/optical
        if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != 3 {
            continue;
        }
        let mut avail = 0u64;
        let mut total = 0u64;
        let mut free = 0u64;
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(root.as_ptr()),
                Some(&mut avail),
                Some(&mut total),
                Some(&mut free),
            )
        };
        if ok.is_ok() && total > 0 {
            out.push(DriveInfo {
                letter: format!("{}:", letter),
                total,
                free,
            });
        }
    }
    out
}

type Walk = jwalk::WalkDirGeneric<((), u64)>;

// Full-drive walk: jwalk parallelizes directory reads AND the per-file
// metadata stat (done in process_read_dir, on worker threads).
pub fn scan_root<F: FnMut(u64, u64)>(root: &Path, mut on_progress: F) -> SpaceScan {
    let mut dirs: HashMap<PathBuf, u64> = HashMap::new();
    let mut files = 0u64;
    let mut bytes = 0u64;
    // Bounded min-heap: smallest of the current top-N sits at the top so we can
    // cheaply reject files that can't make the cut, only cloning paths that do.
    let mut top: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
    let mut last = std::time::Instant::now();

    let walker = Walk::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir(|_depth, _path, _state, children| {
            for child in children.iter_mut().flatten() {
                if child.file_type.is_file() {
                    child.client_state = child.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        });

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type.is_file() {
            continue;
        }
        let size = entry.client_state;
        files += 1;
        bytes += size;
        let path = entry.path();
        if top.len() < TOP_FILES {
            top.push(Reverse((size, path.clone())));
        } else if top.peek().is_some_and(|Reverse((min, _))| size > *min) {
            top.pop();
            top.push(Reverse((size, path.clone())));
        }
        let mut p: &Path = &path;
        while let Some(parent) = p.parent() {
            *dirs.entry(parent.to_path_buf()).or_insert(0) += size;
            if parent == root {
                break;
            }
            p = parent;
        }
        if last.elapsed().as_millis() > 400 {
            on_progress(files, bytes);
            last = std::time::Instant::now();
        }
    }

    let mut biggest: Vec<FolderEntry> = top
        .into_iter()
        .map(|Reverse((b, p))| FolderEntry {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string()),
            path: p.display().to_string(),
            bytes: b,
        })
        .collect();
    biggest.sort_by_key(|e| std::cmp::Reverse(e.bytes));

    SpaceScan {
        root: root.to_path_buf(),
        dirs,
        biggest,
        files,
        bytes,
    }
}

pub fn children_of(scan: &SpaceScan, dir: &Path) -> Vec<FolderEntry> {
    let mut out: Vec<FolderEntry> = scan
        .dirs
        .iter()
        .filter(|(p, _)| p.parent() == Some(dir))
        .map(|(p, b)| FolderEntry {
            path: p.display().to_string(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string()),
            bytes: *b,
        })
        .collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.bytes));
    out.truncate(100);
    out
}

// "Hotspots": folders >=1 GB where no single child holds >=80% of the size —
// i.e. the actual concentration points, not the container chain above them.
pub fn hotspots(scan: &SpaceScan) -> Vec<FolderEntry> {
    let mut max_child: HashMap<&Path, u64> = HashMap::new();
    for (p, &b) in &scan.dirs {
        if let Some(parent) = p.parent() {
            let e = max_child.entry(parent).or_insert(0);
            if b > *e {
                *e = b;
            }
        }
    }
    let root_depth = scan.root.components().count();
    let mut out: Vec<FolderEntry> = scan
        .dirs
        .iter()
        .filter(|(p, &b)| {
            b >= 1_000_000_000
                && p.components().count() > root_depth
                && max_child.get(p.as_path()).copied().unwrap_or(0) * 5 < b * 4
        })
        .map(|(p, b)| FolderEntry {
            path: p.display().to_string(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string()),
            bytes: *b,
        })
        .collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.bytes));
    out.truncate(20);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_aggregates_sizes_up_the_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("a").join("b");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("f1.bin"), vec![0u8; 100]).unwrap();
        fs::write(tmp.path().join("a").join("f2.bin"), vec![0u8; 50]).unwrap();

        let scan = scan_root(tmp.path(), |_, _| {});
        assert_eq!(scan.bytes, 150);
        assert_eq!(scan.files, 2);
        assert_eq!(scan.dirs.get(&tmp.path().join("a")).copied(), Some(150));
        assert_eq!(scan.dirs.get(&sub).copied(), Some(100));

        let kids = children_of(&scan, tmp.path());
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].bytes, 150);
    }
}
