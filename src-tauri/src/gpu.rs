// Old GPU driver installer leftovers (C:\NVIDIA, C:\AMD).
// Only folders *inside* those roots are ever listed (never the root itself), and
// the folder for the installed driver version is always kept. If the installed
// version can't be read from the registry, nothing is listed at all.

use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

pub const NVIDIA_ROOT: &str = "C:\\NVIDIA";
pub const AMD_ROOT: &str = "C:\\AMD";

// Display adapters device class.
const DISPLAY_CLASS: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";

/// True when `p` is strictly below `root` (case-insensitive, no `..` segments).
pub fn inside(p: &Path, root: &Path) -> bool {
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let norm = |x: &Path| x.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    let pl = norm(p);
    let rl = norm(root);
    let rl = rl.trim_end_matches('\\');
    pl.len() > rl.len() + 1 && pl.starts_with(rl) && pl.as_bytes()[rl.len()] == b'\\'
}

// (provider, DriverVersion, RadeonSoftwareVersion) for each display adapter driver.
fn display_drivers() -> Vec<(String, String, Option<String>)> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let Ok(class) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(DISPLAY_CLASS) else {
        return Vec::new();
    };
    class
        .enum_keys()
        .flatten()
        .filter_map(|k| class.open_subkey(&k).ok())
        .filter_map(|key| {
            let provider: String = key.get_value("ProviderName").ok()?;
            let version: String = key.get_value("DriverVersion").ok()?;
            Some((
                provider,
                version,
                key.get_value("RadeonSoftwareVersion").ok(),
            ))
        })
        .collect()
}

/// "32.0.15.6636" -> "566.36": NVIDIA's public version is the last five digits.
pub fn nvidia_public_version(driver_version: &str) -> Option<String> {
    let parts: Vec<&str> = driver_version.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let digits = format!("{}{:0>4}", parts[2], parts[3]);
    if digits.len() < 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let tail = &digits[digits.len() - 5..];
    Some(format!("{}.{}", &tail[..3], &tail[3..]))
}

pub fn installed_nvidia_version() -> Option<String> {
    display_drivers()
        .into_iter()
        .find(|(p, _, _)| p.to_ascii_lowercase().contains("nvidia"))
        .and_then(|(_, v, _)| nvidia_public_version(&v))
}

pub fn installed_amd_version() -> Option<String> {
    display_drivers()
        .into_iter()
        .find(|(p, _, _)| {
            let p = p.to_ascii_lowercase();
            p.contains("advanced micro devices") || p.contains("amd")
        })
        .and_then(|(_, _, radeon)| radeon)
        .filter(|v| !v.trim().is_empty())
}

// Real subfolders only: junctions and symlinks are never listed.
fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
        .map(|e| e.path())
        .collect()
}

fn name_lc(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn modified(p: &Path) -> SystemTime {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

// Drop the folder(s) to keep: names containing the installed version, or — when
// none match — the most recently modified one (most likely the installed driver).
fn without_kept(mut dirs: Vec<PathBuf>, version: &str) -> Vec<PathBuf> {
    let v = version.to_ascii_lowercase();
    if dirs.iter().any(|p| name_lc(p).contains(&v)) {
        dirs.retain(|p| !name_lc(p).contains(&v));
    } else if let Some(newest) = dirs.iter().max_by_key(|p| modified(p)).cloned() {
        dirs.retain(|p| *p != newest);
    }
    dirs
}

/// Leftover folders under C:\NVIDIA: every DisplayDriver\<version> except the
/// installed one, plus the other unpacked installers (GFExperience, …).
pub fn nvidia_targets(root: &Path, installed: Option<&str>) -> Vec<PathBuf> {
    let Some(ver) = installed else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for d in subdirs(root) {
        if name_lc(&d) == "displaydriver" {
            out.extend(without_kept(subdirs(&d), ver));
        } else {
            out.push(d);
        }
    }
    out
}

/// Leftover package folders under C:\AMD, keeping the installed Adrenalin version.
pub fn amd_targets(root: &Path, installed: Option<&str>) -> Vec<PathBuf> {
    match installed {
        Some(ver) => without_kept(subdirs(root), ver),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn names(v: &[PathBuf]) -> Vec<String> {
        let mut n: Vec<String> = v.iter().map(|p| name_lc(p)).collect();
        n.sort();
        n
    }

    #[test]
    fn nvidia_version_from_driver_version() {
        assert_eq!(
            nvidia_public_version("32.0.15.6636").as_deref(),
            Some("566.36")
        );
        assert_eq!(
            nvidia_public_version("31.0.15.3623").as_deref(),
            Some("536.23")
        );
        assert_eq!(nvidia_public_version("garbage"), None);
    }

    #[test]
    fn nvidia_keeps_installed_version_and_root() {
        let tmp = tempfile::tempdir().unwrap();
        let dd = tmp.path().join("DisplayDriver");
        for v in ["551.86", "560.94", "566.36"] {
            fs::create_dir_all(dd.join(v).join("Win11")).unwrap();
            fs::write(dd.join(v).join("Win11").join("setup.exe"), b"x").unwrap();
        }
        fs::create_dir_all(tmp.path().join("GFExperience")).unwrap();

        let t = nvidia_targets(tmp.path(), Some("566.36"));
        assert_eq!(names(&t), ["551.86", "560.94", "gfexperience"]);
        assert!(t.iter().all(|p| inside(p, tmp.path())));

        // Unknown installed version: touch nothing.
        assert!(nvidia_targets(tmp.path(), None).is_empty());
    }

    #[test]
    fn amd_keeps_newest_when_no_folder_names_the_version() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("Radeon-Software-Adrenalin-2020-22.5.1");
        let new = tmp.path().join("AMD-Software-Installer");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        let past = filetime::FileTime::from_system_time(
            SystemTime::now() - std::time::Duration::from_secs(90 * 86400),
        );
        filetime::set_file_mtime(&old, past).unwrap();

        let t = amd_targets(tmp.path(), Some("24.10.1"));
        assert_eq!(names(&t), ["radeon-software-adrenalin-2020-22.5.1"]);

        let t = amd_targets(tmp.path(), Some("22.5.1"));
        assert_eq!(names(&t), ["amd-software-installer"]);
    }

    #[test]
    fn inside_rejects_root_siblings_and_parent_hops() {
        let root = Path::new("C:\\NVIDIA");
        assert!(inside(Path::new("C:\\NVIDIA\\DisplayDriver\\551.86"), root));
        assert!(inside(Path::new("c:/nvidia/GFExperience"), root));
        assert!(!inside(Path::new("C:\\NVIDIA"), root));
        assert!(!inside(Path::new("C:\\NVIDIA\\"), root));
        assert!(!inside(Path::new("C:\\NVIDIA2\\x"), root));
        assert!(!inside(Path::new("C:\\NVIDIA\\..\\Windows"), root));
    }
}
