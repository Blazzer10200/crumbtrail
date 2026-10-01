// Read-only discovery for the Space tab: installed games (Steam, Epic), launcher
// icons, and old installers sitting in Downloads. Nothing here deletes anything.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct Game {
    pub name: String,
    pub launcher: &'static str,
    pub path: String,
    pub bytes: u64,
    // Unix seconds; None when the launcher doesn't record it.
    pub last_played: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct Installer {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub age_days: u64,
}

#[derive(Serialize, Clone)]
pub struct Installers {
    pub dir: String,
    pub old: Vec<Installer>,
    pub newer: u64,
}

// Real on-disk casing without the \\?\ prefix, so paths match the drive walk.
fn real_path(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    let s = c.display().to_string();
    Some(PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s)))
}

// One `"key"  "value"` line from a Valve KeyValues (.vdf / .acf) file.
fn kv(line: &str) -> Option<(String, String)> {
    let mut parts = line.split('"').skip(1).step_by(2);
    let k = parts.next()?;
    let v = parts.next()?;
    Some((k.to_string(), v.replace("\\\\", "\\")))
}

/// First value of each top-level-looking key in an appmanifest.
pub fn parse_acf(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (k, v) in text.lines().filter_map(kv) {
        out.entry(k.to_ascii_lowercase()).or_insert(v);
    }
    out
}

fn steam_games() -> Vec<Game> {
    let Some(root) = crate::categories::steam_root() else {
        return Vec::new();
    };
    let mut libs = vec![root.clone()];
    if let Ok(text) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        libs.extend(
            text.lines()
                .filter_map(kv)
                .filter(|(k, _)| k.eq_ignore_ascii_case("path"))
                .map(|(_, v)| PathBuf::from(v)),
        );
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for lib in libs.iter().filter_map(|l| real_path(l)) {
        if !seen.insert(lib.display().to_string().to_ascii_lowercase()) {
            continue;
        }
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else {
            continue;
        };
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().to_string();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(e.path()) else {
                continue;
            };
            let m = parse_acf(&text);
            let (Some(name), Some(dir)) = (m.get("name"), m.get("installdir")) else {
                continue;
            };
            // Redistributables and compatibility tools aren't games.
            if m.get("appid").map(String::as_str) == Some("228980")
                || name.starts_with("Steamworks")
                || name.starts_with("Steam Linux Runtime")
                || name.starts_with("Proton")
            {
                continue;
            }
            let Some(path) = real_path(&apps.join("common").join(dir)) else {
                continue;
            };
            out.push(Game {
                name: name.clone(),
                launcher: "steam",
                path: path.display().to_string(),
                bytes: m
                    .get("sizeondisk")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                last_played: m
                    .get("lastplayed")
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|&t| t > 0),
            });
        }
    }
    out
}

fn epic_games() -> Vec<Game> {
    let pd = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"));
    let dir = pd
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for e in rd.flatten() {
        if e.path().extension().and_then(|x| x.to_str()) != Some("item") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("");
        // DLC manifests point at their parent game; skip them and half-finished installs.
        let is_dlc = !s("MainGameAppName").is_empty() && s("MainGameAppName") != s("AppName");
        if is_dlc || v.get("bIsIncompleteInstall").and_then(|x| x.as_bool()) == Some(true) {
            continue;
        }
        let Some(path) = real_path(Path::new(s("InstallLocation"))) else {
            continue;
        };
        if !seen.insert(path.display().to_string().to_ascii_lowercase()) {
            continue;
        }
        out.push(Game {
            name: s("DisplayName").to_string(),
            launcher: "epic",
            path: path.display().to_string(),
            bytes: v.get("InstallSize").and_then(|x| x.as_u64()).unwrap_or(0),
            last_played: None,
        });
    }
    out
}

pub fn games() -> Vec<Game> {
    let mut g = steam_games();
    g.extend(epic_games());
    g
}

fn epic_exe() -> Option<PathBuf> {
    let mut bases = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(p) = std::env::var_os(var) {
            bases.push(PathBuf::from(p));
        }
    }
    bases.into_iter().find_map(|b| {
        ["Win64", "Win32"].into_iter().find_map(|arch| {
            let p = b
                .join("Epic Games")
                .join("Launcher")
                .join("Portal")
                .join("Binaries")
                .join(arch)
                .join("EpicGamesLauncher.exe");
            p.is_file().then_some(p)
        })
    })
}

/// Launcher icons as PNG data URLs, pulled from each launcher's own exe.
/// Launchers whose exe or icon can't be read are simply missing (UI shows a letter).
pub fn launcher_icons() -> HashMap<&'static str, String> {
    let mut out = HashMap::new();
    let steam = crate::categories::steam_root().map(|r| r.join("steam.exe"));
    for (key, exe) in [("steam", steam), ("epic", epic_exe())] {
        if let Some(url) = exe
            .filter(|p| p.is_file())
            .and_then(|p| icon::data_url(&p, 64))
        {
            out.insert(key, url);
        }
    }
    out
}

mod icon {
    use base64::Engine;
    use std::path::Path;
    use windows::core::HSTRING;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::Shell::SHDefExtractIconW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    pub fn data_url(exe: &Path, size: u32) -> Option<String> {
        let (w, h, rgba) = extract(exe, size)?;
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().ok()?;
            wr.write_image_data(&rgba).ok()?;
        }
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png_bytes)
        ))
    }

    fn extract(exe: &Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
        let file = HSTRING::from(exe.as_os_str());
        let mut hicon = HICON::default();
        let hr = unsafe { SHDefExtractIconW(&file, 0, 0, Some(&mut hicon), None, size) };
        if hr.is_err() || hicon.is_invalid() {
            return None;
        }
        let px = unsafe { pixels(hicon) };
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        px
    }

    // Colour bitmap of the icon as top-down RGBA.
    unsafe fn pixels(hicon: HICON) -> Option<(u32, u32, Vec<u8>)> {
        let mut ii = ICONINFO::default();
        GetIconInfo(hicon, &mut ii).ok()?;
        let cleanup = |ii: &ICONINFO| {
            let _ = DeleteObject(ii.hbmColor.into());
            let _ = DeleteObject(ii.hbmMask.into());
        };
        if ii.hbmColor.is_invalid() {
            cleanup(&ii);
            return None;
        }
        let mut bmp = BITMAP::default();
        let got = GetObjectW(
            ii.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut BITMAP as *mut core::ffi::c_void),
        );
        let (w, h) = (bmp.bmWidth, bmp.bmHeight);
        if got == 0 || w <= 0 || h <= 0 {
            cleanup(&ii);
            return None;
        }
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // negative = top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            ii.hbmColor,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bi,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        cleanup(&ii);
        if lines == 0 {
            return None;
        }
        // BGRA -> RGBA. Old icons carry no alpha channel at all: treat them as opaque.
        let has_alpha = buf.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
        for p in buf.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            }
        }
        Some((w as u32, h as u32, buf))
    }
}

pub fn downloads_dir() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG};
    let known = unsafe {
        SHGetKnownFolderPath(&FOLDERID_Downloads, KNOWN_FOLDER_FLAG(0), None)
            .ok()
            .map(|p| {
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
                s
            })
    };
    known
        .flatten()
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("Downloads")))
        .filter(|p| p.is_dir())
}

const INSTALLER_EXT: [&str; 3] = ["exe", "msi", "zip"];
const OLD_DAYS: u64 = 30;

/// .exe / .msi / .zip files directly in `dir`, split at 30 days since last modified.
pub fn installers_in(dir: &Path, now: SystemTime) -> Installers {
    let mut old = Vec::new();
    let mut newer = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_file() || ft.is_symlink() {
                continue;
            }
            let path = e.path();
            let ext = path
                .extension()
                .map(|x| x.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if !INSTALLER_EXT.contains(&ext.as_str()) {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            let age = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
                .map(|d| d.as_secs() / 86_400)
                .unwrap_or(0);
            if age > OLD_DAYS {
                old.push(Installer {
                    name: e.file_name().to_string_lossy().to_string(),
                    path: path.display().to_string(),
                    bytes: meta.len(),
                    age_days: age,
                });
            } else {
                newer += 1;
            }
        }
    }
    old.sort_by_key(|i| std::cmp::Reverse(i.bytes));
    Installers {
        dir: dir.display().to_string(),
        old,
        newer,
    }
}

pub fn installers() -> Option<Installers> {
    downloads_dir().map(|d| installers_in(&d, SystemTime::now()))
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn acf_fields_are_read() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1145350\"\n\t\"name\"\t\t\"Hades II\"\n\t\"installdir\"\t\t\"Hades II\"\n\t\"LastPlayed\"\t\t\"1727600000\"\n\t\"SizeOnDisk\"\t\t\"5690000000\"\n\t\"UserConfig\"\n\t{\n\t\t\"name\"\t\t\"other\"\n\t}\n}";
        let m = parse_acf(acf);
        assert_eq!(m["name"], "Hades II");
        assert_eq!(m["sizeondisk"], "5690000000");
        assert_eq!(m["lastplayed"], "1727600000");
        assert_eq!(
            kv("\t\"path\"\t\t\"D:\\\\SteamLibrary\"").unwrap().1,
            "D:\\SteamLibrary"
        );
    }

    #[test]
    fn installers_split_at_thirty_days() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("OBS-Setup.exe");
        fs::write(&old, vec![0u8; 30]).unwrap();
        let t = filetime::FileTime::from_system_time(
            SystemTime::now() - Duration::from_secs(45 * 86_400),
        );
        filetime::set_file_mtime(&old, t).unwrap();
        fs::write(tmp.path().join("ChromeSetup.exe"), vec![0u8; 5]).unwrap();
        fs::write(tmp.path().join("photo.jpg"), vec![0u8; 5]).unwrap();

        let r = installers_in(tmp.path(), SystemTime::now());
        assert_eq!(r.old.len(), 1);
        assert_eq!(r.old[0].name, "OBS-Setup.exe");
        assert!(r.old[0].age_days >= 44);
        assert_eq!(r.newer, 1);
    }
}
