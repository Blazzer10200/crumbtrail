use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// How many of the largest individual files to keep during a scan.
const TOP_FILES: usize = 200;
// Biggest files kept per file type (File types view).
const TYPE_TOP: usize = 12;
// Unreadable folders kept by path (the count covers all of them).
const UNREADABLE_KEEP: usize = 64;

// File types view buckets, in display order.
pub const TYPE_KEYS: [&str; 7] = [
    "video",
    "game",
    "installer",
    "archive",
    "image",
    "doc",
    "other",
];
const GAME: usize = 1;

#[derive(Default)]
pub struct SpaceState(pub Mutex<Option<SpaceScan>>);

pub struct SpaceScan {
    pub root: PathBuf,
    pub dirs: HashMap<PathBuf, u64>,
    pub biggest: Vec<FolderEntry>,
    pub files: u64,
    pub bytes: u64,
    pub types: [u64; 7],
    pub type_top: Vec<Vec<FolderEntry>>,
    pub unreadable: Vec<String>,
    pub unreadable_count: u64,
    // Drive used bytes at scan time (drive-root scans only) — What changed hero number.
    pub used_bytes: Option<u64>,
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
type TopHeap = BinaryHeap<Reverse<(u64, PathBuf)>>;

// Bounded min-heap: smallest of the current top-N sits at the top so we can
// cheaply reject files that can't make the cut, only cloning paths that do.
fn offer(heap: &mut TopHeap, cap: usize, size: u64, path: &Path) {
    if heap.len() < cap {
        heap.push(Reverse((size, path.to_path_buf())));
    } else if heap.peek().is_some_and(|Reverse((min, _))| size > *min) {
        heap.pop();
        heap.push(Reverse((size, path.to_path_buf())));
    }
}

fn entry_of(p: &Path, bytes: u64) -> FolderEntry {
    FolderEntry {
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.display().to_string()),
        path: p.display().to_string(),
        bytes,
    }
}

fn sorted(heap: TopHeap) -> Vec<FolderEntry> {
    let mut v: Vec<FolderEntry> = heap
        .into_iter()
        .map(|Reverse((b, p))| entry_of(&p, b))
        .collect();
    v.sort_by_key(|e| Reverse(e.bytes));
    v
}

/// Extension bucket for the File types view (game files are tagged by folder instead).
pub fn type_of(name: &str) -> usize {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match ext {
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v" => 0,
        "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" => 2,
        "exe" if lower.contains("setup") || lower.contains("install") => 2,
        "zip" | "7z" | "rar" | "iso" | "vhdx" | "vhd" | "tar" | "gz" | "tgz" | "bz2" | "xz" => 3,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "heic" | "tif" | "tiff" | "psd"
        | "raw" | "arw" | "cr2" | "cr3" | "nef" | "dng" => 4,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "odt" | "rtf"
        | "csv" | "md" => 5,
        _ => 6,
    }
}

// Full-drive walk: jwalk parallelizes directory reads AND the per-file
// metadata stat (done in process_read_dir, on worker threads).
// `games` holds installed game folders (exact on-disk casing) for the "game" bucket.
pub fn scan_root<F: FnMut(u64, u64)>(
    root: &Path,
    games: &HashSet<PathBuf>,
    mut on_progress: F,
) -> SpaceScan {
    let mut dirs: HashMap<PathBuf, u64> = HashMap::new();
    let mut files = 0u64;
    let mut bytes = 0u64;
    let mut top: TopHeap = BinaryHeap::new();
    let mut types = [0u64; 7];
    let mut type_heaps: Vec<TopHeap> = (0..7).map(|_| BinaryHeap::new()).collect();
    let mut unreadable: Vec<String> = Vec::new();
    let mut unreadable_count = 0u64;
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

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                if e.io_error().map(|io| io.kind()) == Some(std::io::ErrorKind::PermissionDenied) {
                    unreadable_count += 1;
                    if unreadable.len() < UNREADABLE_KEEP {
                        if let Some(p) = e.path() {
                            unreadable.push(p.display().to_string());
                        }
                    }
                }
                continue;
            }
        };
        if !entry.file_type.is_file() {
            continue;
        }
        let size = entry.client_state;
        files += 1;
        bytes += size;
        let path = entry.path();
        offer(&mut top, TOP_FILES, size, &path);
        let mut in_game = false;
        let mut p: &Path = &path;
        while let Some(parent) = p.parent() {
            *dirs.entry(parent.to_path_buf()).or_insert(0) += size;
            if !in_game && !games.is_empty() && games.contains(parent) {
                in_game = true;
            }
            if parent == root {
                break;
            }
            p = parent;
        }
        let t = if in_game {
            GAME
        } else {
            type_of(&entry.file_name.to_string_lossy())
        };
        types[t] += size;
        offer(&mut type_heaps[t], TYPE_TOP, size, &path);
        if last.elapsed().as_millis() > 400 {
            on_progress(files, bytes);
            last = std::time::Instant::now();
        }
    }

    // Shallowest paths first: "C:\System Volume Information" reads better than its children.
    unreadable.sort_by_key(|p| (p.matches('\\').count(), p.len()));

    SpaceScan {
        root: root.to_path_buf(),
        dirs,
        biggest: sorted(top),
        files,
        bytes,
        types,
        type_top: type_heaps.into_iter().map(sorted).collect(),
        unreadable,
        unreadable_count,
        used_bytes: None,
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

        let scan = scan_root(tmp.path(), &HashSet::new(), |_, _| {});
        assert_eq!(scan.bytes, 150);
        assert_eq!(scan.files, 2);
        assert_eq!(scan.dirs.get(&tmp.path().join("a")).copied(), Some(150));
        assert_eq!(scan.dirs.get(&sub).copied(), Some(100));

        let kids = children_of(&scan, tmp.path());
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].bytes, 150);
    }

    #[test]
    fn file_types_bucket_by_extension_and_game_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Games").join("Hades II");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("content.pak"), vec![0u8; 70]).unwrap();
        fs::write(game.join("intro.mp4"), vec![0u8; 30]).unwrap();
        fs::write(tmp.path().join("trip.mkv"), vec![0u8; 20]).unwrap();
        fs::write(tmp.path().join("OBS-Setup.exe"), vec![0u8; 10]).unwrap();
        fs::write(tmp.path().join("notes.bin"), vec![0u8; 5]).unwrap();

        let games: HashSet<PathBuf> = [game.clone()].into_iter().collect();
        let scan = scan_root(tmp.path(), &games, |_, _| {});
        assert_eq!(scan.types, [20, 100, 10, 0, 0, 0, 5]);
        assert_eq!(scan.type_top[GAME][0].name, "content.pak");
    }
}
