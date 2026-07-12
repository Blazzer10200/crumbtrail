use crate::categories::{Category, Kind};
use serde::Serialize;
use std::fs;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

#[derive(Serialize, Clone)]
pub struct ScanResult {
    pub id: &'static str,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Serialize, Clone)]
pub struct CleanResult {
    pub id: &'static str,
    pub freed_bytes: u64,
    pub deleted: u64,
    pub skipped: u64,
}

fn too_new(meta: &fs::Metadata, age_hours: Option<u64>) -> bool {
    let Some(h) = age_hours else { return false };
    match meta.modified() {
        Ok(m) => {
            SystemTime::now().duration_since(m).unwrap_or(Duration::ZERO)
                < Duration::from_secs(h * 3600)
        }
        // Can't read the timestamp — leave the file alone.
        Err(_) => true,
    }
}

fn name_matches(name: &str, prefixes: Option<&[&str]>) -> bool {
    match prefixes {
        None => true,
        Some(ps) => {
            let lower = name.to_ascii_lowercase();
            ps.iter().any(|p| lower.starts_with(p))
        }
    }
}

pub fn scan_category(cat: &Category) -> ScanResult {
    match cat.kind {
        Kind::RecycleBin => {
            let (bytes, files) = recycle_bin_query();
            ScanResult { id: cat.id, bytes, files }
        }
        Kind::Files => {
            let mut bytes = 0u64;
            let mut files = 0u64;
            for root in &cat.paths {
                for entry in WalkDir::new(root)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if !entry.file_type().is_file() || entry.path_is_symlink() {
                        continue;
                    }
                    if !name_matches(&entry.file_name().to_string_lossy(), cat.file_prefixes) {
                        continue;
                    }
                    if let Ok(meta) = entry.metadata() {
                        if too_new(&meta, cat.age_hours) {
                            continue;
                        }
                        bytes += meta.len();
                        files += 1;
                    }
                }
            }
            ScanResult { id: cat.id, bytes, files }
        }
    }
}

pub fn clean_category(cat: &Category, log: &mut Vec<String>) -> CleanResult {
    match cat.kind {
        Kind::RecycleBin => {
            let (bytes, files) = recycle_bin_query();
            let ok = recycle_bin_empty();
            log.push(format!(
                "[{}] empty recycle bin: {} bytes / {} items, ok={}",
                cat.id, bytes, files, ok
            ));
            if ok {
                CleanResult { id: cat.id, freed_bytes: bytes, deleted: files, skipped: 0 }
            } else {
                CleanResult { id: cat.id, freed_bytes: 0, deleted: 0, skipped: files }
            }
        }
        Kind::Files => {
            let mut freed = 0u64;
            let mut deleted = 0u64;
            let mut skipped = 0u64;
            for root in &cat.paths {
                for entry in WalkDir::new(root)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if !entry.file_type().is_file() || entry.path_is_symlink() {
                        continue;
                    }
                    if !name_matches(&entry.file_name().to_string_lossy(), cat.file_prefixes) {
                        continue;
                    }
                    let Ok(meta) = entry.metadata() else {
                        skipped += 1;
                        continue;
                    };
                    if too_new(&meta, cat.age_hours) {
                        continue;
                    }
                    match fs::remove_file(entry.path()) {
                        Ok(()) => {
                            freed += meta.len();
                            deleted += 1;
                            log.push(format!("[{}] del {}", cat.id, entry.path().display()));
                        }
                        // Locked / in use / permission denied — skip, never force.
                        Err(_) => skipped += 1,
                    }
                }
                // Prune now-empty subdirectories (never the root itself).
                // remove_dir fails on non-empty dirs, which is exactly what we want.
                if cat.file_prefixes.is_none() {
                    for entry in WalkDir::new(root)
                        .follow_links(false)
                        .contents_first(true)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_dir()
                            && !entry.path_is_symlink()
                            && entry.path() != root.as_path()
                        {
                            let _ = fs::remove_dir(entry.path());
                        }
                    }
                }
            }
            log.push(format!(
                "[{}] freed {} bytes, deleted {}, skipped {}",
                cat.id, freed, deleted, skipped
            ));
            CleanResult { id: cat.id, freed_bytes: freed, deleted, skipped }
        }
    }
}

fn recycle_bin_query() -> (u64, u64) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    unsafe {
        let mut info = SHQUERYRBINFO {
            cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
            i64Size: 0,
            i64NumItems: 0,
        };
        if SHQueryRecycleBinW(PCWSTR::null(), &mut info).is_ok() {
            (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64)
        } else {
            (0, 0)
        }
    }
}

fn recycle_bin_empty() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    };
    unsafe {
        SHEmptyRecycleBinW(
            None,
            PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::{Category, Kind};
    use std::path::{Path, PathBuf};

    fn cat(
        paths: Vec<PathBuf>,
        age_hours: Option<u64>,
        prefixes: Option<&'static [&'static str]>,
    ) -> Category {
        Category {
            id: "test",
            name: "test",
            description: "",
            module: "core",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths,
            age_hours,
            file_prefixes: prefixes,
        }
    }

    fn old_file(dir: &Path, name: &str, bytes: usize) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, vec![0u8; bytes]).unwrap();
        let t = filetime::FileTime::from_system_time(
            SystemTime::now() - Duration::from_secs(72 * 3600),
        );
        filetime::set_file_mtime(&p, t).unwrap();
        p
    }

    #[test]
    fn age_threshold_keeps_fresh_files() {
        let tmp = tempfile::tempdir().unwrap();
        old_file(tmp.path(), "old.tmp", 100);
        fs::write(tmp.path().join("fresh.tmp"), vec![0u8; 50]).unwrap();
        let c = cat(vec![tmp.path().to_path_buf()], Some(48), None);

        let scan = scan_category(&c);
        assert_eq!((scan.bytes, scan.files), (100, 1));

        let res = clean_category(&c, &mut Vec::new());
        assert_eq!((res.freed_bytes, res.deleted, res.skipped), (100, 1, 0));
        assert!(tmp.path().join("fresh.tmp").exists());
        assert!(!tmp.path().join("old.tmp").exists());
    }

    #[test]
    fn prefix_filter_only_touches_matching_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("thumbcache_256.db"), vec![0u8; 10]).unwrap();
        fs::write(tmp.path().join("settings.dat"), vec![0u8; 20]).unwrap();
        let c = cat(
            vec![tmp.path().to_path_buf()],
            None,
            Some(&["thumbcache_"]),
        );

        let scan = scan_category(&c);
        assert_eq!((scan.bytes, scan.files), (10, 1));

        let res = clean_category(&c, &mut Vec::new());
        assert_eq!(res.deleted, 1);
        assert!(tmp.path().join("settings.dat").exists());
        assert!(!tmp.path().join("thumbcache_256.db").exists());
        assert!(tmp.path().exists());
    }

    #[test]
    fn prunes_empty_subdirs_but_never_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("junk.bin"), vec![0u8; 5]).unwrap();
        let c = cat(vec![tmp.path().to_path_buf()], None, None);

        let res = clean_category(&c, &mut Vec::new());
        assert_eq!((res.deleted, res.skipped), (1, 0));
        assert!(!tmp.path().join("a").exists());
        assert!(tmp.path().exists());
    }

    #[test]
    fn symlinked_dir_contents_are_never_touched() {
        let outside = tempfile::tempdir().unwrap();
        let precious = outside.path().join("precious.txt");
        fs::write(&precious, b"do not delete").unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("link");
        // Needs Developer Mode or elevation on Windows — skip quietly if unavailable.
        if std::os::windows::fs::symlink_dir(outside.path(), &link).is_err() {
            return;
        }

        let c = cat(vec![tmp.path().to_path_buf()], None, None);
        let scan = scan_category(&c);
        assert_eq!(scan.files, 0);

        clean_category(&c, &mut Vec::new());
        assert!(precious.exists());
        assert!(outside.path().exists());
    }
}

pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut TOKEN_ELEVATION as *mut core::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
}
