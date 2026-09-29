use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Files,
    RecycleBin,
}

#[derive(Clone)]
pub struct Category {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub module: &'static str, // core | gaming | dev
    pub risk: &'static str,   // safe | care
    pub needs_admin: bool,
    pub kind: Kind,
    pub paths: Vec<PathBuf>,
    // Files younger than this are always left alone (may belong to running apps).
    pub age_hours: Option<u64>,
    // When set, only files whose lowercased name starts with one of these are touched,
    // and directories are never removed (used for thumbnail/icon caches).
    pub file_prefixes: Option<&'static [&'static str]>,
}

#[derive(Serialize, Clone)]
pub struct CategoryInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub module: &'static str,
    pub risk: &'static str,
    pub needs_admin: bool,
    pub available: bool,
}

impl Category {
    pub fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: self.id,
            name: self.name,
            description: self.description,
            module: self.module,
            risk: self.risk,
            needs_admin: self.needs_admin,
            available: self.kind == Kind::RecycleBin || !self.paths.is_empty(),
        }
    }
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

fn existing(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

fn chromium_caches(user_data: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&user_data) else {
        return out;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name == "Default" || name.starts_with("Profile ") {
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                let p = e.path().join(sub);
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn firefox_caches(local: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(local.join("Mozilla").join("Firefox").join("Profiles"))
    else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path().join("cache2");
        if p.is_dir() {
            out.push(p);
        }
    }
    out
}

fn steam_root() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("Software\\Valve\\Steam") {
        if let Ok(path) = key.get_value::<String, _>("SteamPath") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    let fallback = PathBuf::from("C:/Program Files (x86)/Steam");
    fallback.is_dir().then_some(fallback)
}

pub fn build_categories() -> Vec<Category> {
    let local = env_path("LOCALAPPDATA").unwrap_or_else(|| PathBuf::from("C:/Windows/Temp"));
    let profile = env_path("USERPROFILE").unwrap_or_else(|| PathBuf::from("C:/Users/Public"));
    let windir = env_path("SystemRoot").unwrap_or_else(|| PathBuf::from("C:/Windows"));
    let user_temp = env_path("TEMP").unwrap_or_else(|| local.join("Temp"));

    let mut browser_paths = chromium_caches(local.join("Google").join("Chrome").join("User Data"));
    browser_paths.extend(chromium_caches(
        local.join("Microsoft").join("Edge").join("User Data"),
    ));
    browser_paths.extend(firefox_caches(&local));

    let mut citizenfx = Vec::new();
    for app in ["FiveM/FiveM.app", "RedM/RedM.app"] {
        for sub in ["data/cache", "data/server-cache", "data/server-cache-priv"] {
            citizenfx.push(local.join(app).join(sub));
        }
    }

    vec![
        Category {
            id: "user_temp",
            name: "User temp files",
            description: "Temporary files apps left behind (older than 48h)",
            module: "core",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![user_temp]),
            age_hours: Some(48),
            file_prefixes: None,
        },
        Category {
            id: "system_temp",
            name: "System temp files",
            description: "Windows-wide temp folder (older than 48h)",
            module: "core",
            risk: "safe",
            needs_admin: true,
            kind: Kind::Files,
            paths: existing(vec![windir.join("Temp")]),
            age_hours: Some(48),
            file_prefixes: None,
        },
        Category {
            id: "windows_update",
            name: "Windows Update leftovers",
            description: "Already-installed update downloads. Windows re-downloads if ever needed",
            module: "core",
            risk: "care",
            needs_admin: true,
            kind: Kind::Files,
            paths: existing(vec![windir.join("SoftwareDistribution").join("Download")]),
            age_hours: Some(240),
            file_prefixes: None,
        },
        Category {
            id: "browser_cache",
            name: "Browser caches",
            description: "Chrome, Edge and Firefox page caches. Pages reload a bit slower once",
            module: "core",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: browser_paths,
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "thumbnails",
            name: "Thumbnail & icon caches",
            description: "Explorer preview caches. Windows rebuilds them as you browse folders",
            module: "core",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![local
                .join("Microsoft")
                .join("Windows")
                .join("Explorer")]),
            age_hours: None,
            file_prefixes: Some(&["thumbcache_", "iconcache_"]),
        },
        Category {
            id: "crash_dumps",
            name: "Crash dumps & error reports",
            description: "Old crash dumps and Windows Error Reporting queues",
            module: "core",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![
                local.join("CrashDumps"),
                local
                    .join("Microsoft")
                    .join("Windows")
                    .join("WER")
                    .join("ReportQueue"),
                local
                    .join("Microsoft")
                    .join("Windows")
                    .join("WER")
                    .join("ReportArchive"),
            ]),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "recycle_bin",
            name: "Recycle Bin",
            description: "Empties the Recycle Bin on all drives. Gone means gone",
            module: "core",
            risk: "care",
            needs_admin: false,
            kind: Kind::RecycleBin,
            paths: Vec::new(),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "dx_shader",
            name: "DirectX shader cache",
            description: "Rebuilds automatically. First launch of a game may be slightly slower",
            module: "gaming",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![local.join("D3DSCache")]),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "nvidia_shader",
            name: "NVIDIA shader caches",
            description: "DXCache / GLCache / NV_Cache. Rebuilds automatically while you play",
            module: "gaming",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![
                local.join("NVIDIA").join("DXCache"),
                local.join("NVIDIA").join("GLCache"),
                local.join("NVIDIA Corporation").join("NV_Cache"),
            ]),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "steam_shader",
            name: "Steam shader cache",
            description: "Per-game shader caches. Steam re-downloads/rebuilds them as needed",
            module: "gaming",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(
                steam_root()
                    .map(|s| vec![s.join("steamapps").join("shadercache")])
                    .unwrap_or_default(),
            ),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "citizenfx_cache",
            name: "FiveM / RedM cache",
            description: "Server content caches. Re-downloads on next join (slower first connect)",
            module: "gaming",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(citizenfx),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "npm_cache",
            name: "npm cache",
            description: "Package download cache. npm re-fetches what it needs",
            module: "dev",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![local.join("npm-cache"), profile.join(".npm")]),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "pip_cache",
            name: "pip cache",
            description: "Python package download cache. pip re-fetches what it needs",
            module: "dev",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![local.join("pip").join("cache")]),
            age_hours: None,
            file_prefixes: None,
        },
        Category {
            id: "cargo_cache",
            name: "Cargo registry cache",
            description: "Downloaded .crate archives. Cargo re-fetches what it needs",
            module: "dev",
            risk: "safe",
            needs_admin: false,
            kind: Kind::Files,
            paths: existing(vec![profile.join(".cargo").join("registry").join("cache")]),
            age_hours: None,
            file_prefixes: None,
        },
    ]
}
