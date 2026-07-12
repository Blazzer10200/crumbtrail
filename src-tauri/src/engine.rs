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
