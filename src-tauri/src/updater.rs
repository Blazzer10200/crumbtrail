// Auto-update via Velopack + GitHub Releases. The app checks on launch,
// downloads in the background, and applies on "Restart now" — or quietly when
// the app closes. Dev builds aren't Velopack-installed, so they report
// "unsupported" instead of an error.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use velopack::{sources::GithubSource, UpdateCheck, UpdateInfo, UpdateManager};

const REPO: &str = "https://github.com/Blazzer10200/crumbtrail";

/// Set when the app exits to relaunch as admin: the new instance applies the update itself.
pub static SKIP_APPLY_ON_EXIT: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct UpdateState(pub Mutex<Option<UpdateInfo>>);

fn manager() -> Result<UpdateManager, velopack::Error> {
    UpdateManager::new(GithubSource::new(REPO, None, false), None, None)
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Check {
    None,
    Unsupported,
    Available {
        version: String,
        notes: String,
        size: u64,
    },
    Ready {
        version: String,
    },
}

pub fn check(state: &UpdateState) -> Result<Check, String> {
    let um = match manager() {
        Ok(um) => um,
        Err(velopack::Error::NotInstalled(_)) => return Ok(Check::Unsupported),
        Err(e) => return Err(e.to_string()),
    };
    // Downloaded last session but not applied yet.
    if let Some(p) = um.get_update_pending_restart() {
        return Ok(Check::Ready { version: p.Version });
    }
    match um.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(info) => {
            let t = &info.TargetFullRelease;
            let c = Check::Available {
                version: t.Version.clone(),
                notes: t.NotesMarkdown.clone(),
                size: t.Size,
            };
            *state.0.lock().unwrap() = Some(*info);
            Ok(c)
        }
        UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => Ok(Check::None),
    }
}

/// Emits `update:progress` (0-100), then `update:done` or `update:error`.
pub fn download(app: AppHandle, info: UpdateInfo) {
    std::thread::spawn(move || {
        let res = manager().map_err(|e| e.to_string()).and_then(|um| {
            let (tx, rx) = std::sync::mpsc::channel::<i16>();
            let app2 = app.clone();
            let fwd = std::thread::spawn(move || {
                for p in rx {
                    let _ = app2.emit("update:progress", p);
                }
            });
            let r = um.download_updates(&info, Some(tx)).map_err(|e| e.to_string());
            let _ = fwd.join();
            r
        });
        match res {
            Ok(()) => app.emit("update:done", ()),
            Err(e) => app.emit("update:error", e),
        }
        .ok();
    });
}

/// Closes the app, installs the downloaded update and reopens it.
pub fn apply_and_restart() -> Result<(), String> {
    let um = manager().map_err(|e| e.to_string())?;
    let asset = um
        .get_update_pending_restart()
        .ok_or("No downloaded update to install")?;
    um.apply_updates_and_restart(&asset)
        .map_err(|e| e.to_string())
}

/// On exit: hand a downloaded update to Velopack's updater, which installs it
/// silently once this process is gone. Returns an error only worth logging.
pub fn apply_on_exit() -> Result<(), String> {
    if SKIP_APPLY_ON_EXIT.load(Ordering::SeqCst) {
        return Ok(());
    }
    let Ok(um) = manager() else {
        return Ok(()); // not installed (dev build): nothing to apply
    };
    match um.get_update_pending_restart() {
        Some(asset) => um
            .wait_exit_then_apply_updates(&asset, true, false, Vec::<String>::new())
            .map_err(|e| e.to_string()),
        None => Ok(()),
    }
}
