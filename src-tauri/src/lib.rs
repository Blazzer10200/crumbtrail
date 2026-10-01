mod categories;
mod discover;
mod engine;
mod gpu;
mod schedule;
mod snapshot;
mod space;
mod store;
mod updater;

use categories::{build_categories, CategoryInfo};
use engine::{clean_category, is_elevated, scan_category, CleanResult};
use space::SpaceState;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use updater::UpdateState;

#[tauri::command]
fn get_categories() -> Vec<CategoryInfo> {
    build_categories().iter().map(|c| c.info()).collect()
}

#[tauri::command]
fn elevated() -> bool {
    is_elevated()
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".into())
}

#[tauri::command]
fn scan(app: AppHandle) {
    std::thread::spawn(move || {
        let cats = build_categories();
        let mut handles = Vec::new();
        for cat in cats {
            let app2 = app.clone();
            let id = cat.id;
            handles.push((
                id,
                std::thread::spawn(move || {
                    let res = scan_category(&cat);
                    let _ = app2.emit("scan:result", &res);
                }),
            ));
        }
        // A category that blows up stops only itself; the rest still report.
        let mut failed: Vec<(&str, String)> = Vec::new();
        for (id, h) in handles {
            if let Err(p) = h.join() {
                failed.push((id, panic_text(p)));
            }
        }
        if !failed.is_empty() {
            let mut log = vec![format!(
                "Crumbtrail scan {}",
                chrono::Local::now().to_rfc3339()
            )];
            log.extend(
                failed
                    .iter()
                    .map(|(id, m)| format!("[{id}] scan failed: {m}")),
            );
            let log_path = write_log(&log);
            let _ = app.emit(
                "scan:error",
                serde_json::json!({
                    "ids": failed.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                    "message": failed[0].1,
                    "log_path": log_path,
                }),
            );
        }
        let _ = app.emit("scan:done", ());
    });
}

#[tauri::command]
fn clean(app: AppHandle, ids: Vec<String>) {
    std::thread::spawn(move || {
        let cats = build_categories();
        let admin = is_elevated();
        let mut log: Vec<String> = Vec::new();
        log.push(format!(
            "Crumbtrail run {}",
            chrono::Local::now().to_rfc3339()
        ));
        let mut total = 0u64;
        for cat in cats.iter().filter(|c| ids.iter().any(|id| id == c.id)) {
            if cat.needs_admin && !admin {
                log.push(format!("[{}] skipped — needs admin", cat.id));
                let _ = app.emit(
                    "clean:result",
                    &CleanResult {
                        id: cat.id,
                        freed_bytes: 0,
                        deleted: 0,
                        skipped: 0,
                    },
                );
                continue;
            }
            let res = clean_category(cat, &mut log);
            total += res.freed_bytes;
            let _ = app.emit("clean:result", &res);
        }
        let all_time = match store::add_freed(total) {
            Ok(t) => Some(t),
            Err(e) => {
                log.push(format!("all-time counter not updated: {e}"));
                None
            }
        };
        let log_path = write_log(&log);
        let _ = app.emit(
            "clean:done",
            serde_json::json!({ "total_bytes": total, "log_path": log_path, "all_time": all_time }),
        );
    });
}

#[tauri::command]
fn drives() -> Vec<space::DriveInfo> {
    space::list_drives()
}

// Snapshot list for the picker, each with its net change vs. the current scan.
fn older_snapshots(
    scan: &space::SpaceScan,
    drive: &str,
    skip: Option<&str>,
) -> Vec<serde_json::Value> {
    let base = snapshot::dir();
    let root = scan.root.display().to_string();
    let is_drive = root.trim_end_matches('\\').len() <= 2;
    let root_lc = root.trim_end_matches('\\').to_lowercase();
    let now_root = scan.dirs.get(&scan.root).copied().unwrap_or(scan.bytes);
    snapshot::list_in(&base, drive)
        .into_iter()
        .filter(|m| Some(m.id.as_str()) != skip)
        .map(|m| {
            let net = if is_drive {
                Some(scan.used_bytes.unwrap_or(0) as i64 - m.used_bytes as i64)
            } else {
                snapshot::load_in(&base, &m.id).ok().map(|s| {
                    let then = s
                        .folders
                        .iter()
                        .find(|(p, _)| p.to_lowercase() == root_lc)
                        .map(|(_, b)| *b)
                        .unwrap_or(0);
                    now_root as i64 - then as i64
                })
            };
            serde_json::json!({ "id": m.id, "taken_at": m.taken_at, "used_bytes": m.used_bytes, "net": net })
        })
        .collect()
}

#[tauri::command]
fn space_scan(app: AppHandle, root: String) {
    std::thread::spawn(move || {
        let root = PathBuf::from(root);
        let root_s = root.display().to_string();
        let drive: String = root_s.chars().take(2).collect::<String>().to_uppercase();
        let is_drive = root_s.trim_end_matches('\\').len() <= 2;

        let all_games = discover::games();
        let game_dirs: HashSet<PathBuf> =
            all_games.iter().map(|g| PathBuf::from(&g.path)).collect();
        let app2 = app.clone();
        let mut scan = space::scan_root(&root, &game_dirs, move |files, bytes| {
            let _ = app2.emit(
                "space:progress",
                serde_json::json!({ "files": files, "bytes": bytes }),
            );
        });
        scan.used_bytes = space::list_drives()
            .into_iter()
            .find(|d| d.letter.eq_ignore_ascii_case(&drive))
            .map(|d| d.total.saturating_sub(d.free));

        // Only whole-drive scans become snapshots; folder scans compare against them.
        let (saved, snap_error) = match (is_drive, scan.used_bytes) {
            (true, Some(used)) => {
                let snap = snapshot::from_scan(&scan, &drive, used, discover::unix_now());
                match snapshot::save_in(&snapshot::dir(), &snap) {
                    Ok(m) => (Some(m), None),
                    Err(e) => (None, Some(e)),
                }
            }
            _ => (None, None),
        };
        let older = older_snapshots(&scan, &drive, saved.as_ref().map(|m| m.id.as_str()));

        let games: Vec<&discover::Game> = all_games
            .iter()
            .filter(|g| {
                g.path
                    .get(..2)
                    .is_some_and(|d| d.eq_ignore_ascii_case(&drive))
            })
            .collect();
        let types: Vec<serde_json::Value> = space::TYPE_KEYS
            .iter()
            .zip(scan.types.iter())
            .map(|(k, b)| serde_json::json!({ "key": k, "bytes": b }))
            .collect();
        let type_top: serde_json::Map<String, serde_json::Value> = space::TYPE_KEYS
            .iter()
            .zip(scan.type_top.iter())
            .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
            .collect();

        let payload = serde_json::json!({
            "root": root_s,
            "drive": drive,
            "is_drive": is_drive,
            "files": scan.files,
            "bytes": scan.bytes,
            "used_bytes": scan.used_bytes,
            "hotspots": space::hotspots(&scan),
            "top": space::children_of(&scan, &scan.root),
            "biggest": &scan.biggest,
            "unreadable": { "count": scan.unreadable_count, "paths": scan.unreadable.iter().take(3).collect::<Vec<_>>() },
            "types": types,
            "type_top": type_top,
            "games": games,
            "installers": discover::installers(),
            "snapshots": { "saved": saved, "older": older, "error": snap_error },
        });
        *app.state::<SpaceState>().0.lock().unwrap() = Some(scan);
        let _ = app.emit("space:done", payload);
    });
}

#[tauri::command]
fn space_children(state: State<SpaceState>, dir: String) -> Vec<space::FolderEntry> {
    match &*state.0.lock().unwrap() {
        Some(scan) => space::children_of(scan, Path::new(&dir)),
        None => Vec::new(),
    }
}

#[tauri::command]
fn space_diff(state: State<SpaceState>, id: String) -> Result<snapshot::Diff, String> {
    let old = snapshot::load_in(&snapshot::dir(), &id)?;
    match &*state.0.lock().unwrap() {
        Some(scan) => Ok(snapshot::diff(scan, &old)),
        None => Err("Scan a drive first".into()),
    }
}

// Clean-tab teaser: compares live used space against the last saved snapshot.
#[tauri::command]
fn snapshots(drive: String) -> Vec<snapshot::SnapMeta> {
    snapshot::list_in(&snapshot::dir(), &drive.to_uppercase())
}

#[tauri::command]
fn reveal(path: String) {
    let p = Path::new(&path);
    if p.is_dir() {
        let _ = std::process::Command::new("explorer").arg(&path).spawn();
    } else if p.is_file() {
        // /select, opens the parent folder with the file highlighted.
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path))
            .spawn();
    }
}

#[tauri::command]
fn open_launcher(launcher: String) -> Result<(), String> {
    // Fixed URLs only — never a string from the UI.
    let url = match launcher.as_str() {
        "steam" => "steam://open/games",
        "epic" => "com.epicgames.launcher://store/library",
        other => return Err(format!("unknown launcher {other}")),
    };
    std::process::Command::new("explorer")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn launcher_icons() -> std::collections::HashMap<&'static str, String> {
    discover::launcher_icons()
}

#[tauri::command]
fn relaunch_admin(app: AppHandle) -> Result<(), String> {
    engine::relaunch_elevated()?;
    updater::SKIP_APPLY_ON_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(0);
    Ok(())
}

// Network and schtasks calls block, so these run off the main thread.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn update_check(app: AppHandle) -> Result<updater::Check, String> {
    blocking(move || updater::check(&app.state::<UpdateState>())).await
}

#[tauri::command]
fn update_download(app: AppHandle, state: State<UpdateState>) -> Result<(), String> {
    let info = state
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or("Check for updates first")?;
    updater::download(app, info);
    Ok(())
}

#[tauri::command]
fn update_apply() -> Result<(), String> {
    updater::apply_and_restart()
}

#[tauri::command]
async fn weekly_status() -> Result<schedule::Weekly, String> {
    blocking(|| Ok(schedule::status())).await
}

#[tauri::command]
async fn set_weekly(on: bool) -> Result<schedule::Weekly, String> {
    blocking(move || if on { schedule::enable() } else { schedule::disable() }).await
}

/// `crumbtrail.exe --snapshot`, run by the weekly task: scan the system drive,
/// save a snapshot, exit. No window. Failures go to the log folder.
pub fn run_snapshot_task() {
    let res = (|| -> Result<(), String> {
        let drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".into())
            .to_uppercase();
        let root = PathBuf::from(format!("{drive}\\"));
        // Game folders only affect the file-type split, which snapshots don't keep.
        let scan = space::scan_root(&root, &HashSet::new(), |_, _| {});
        let used = space::list_drives()
            .into_iter()
            .find(|d| d.letter.eq_ignore_ascii_case(&drive))
            .map(|d| d.total.saturating_sub(d.free))
            .ok_or_else(|| format!("drive {drive} not found"))?;
        let snap = snapshot::from_scan(&scan, &drive, used, discover::unix_now());
        snapshot::save_in(&snapshot::dir(), &snap).map(|_| ())
    })();
    if let Err(e) = res {
        write_log(&[
            format!("Crumbtrail weekly snapshot {}", chrono::Local::now().to_rfc3339()),
            format!("failed: {e}"),
        ]);
    }
}

/// Velopack uninstall hook: don't leave the weekly task behind.
pub fn remove_weekly_task() {
    let _ = schedule::disable();
}

#[tauri::command]
fn all_time_freed() -> u64 {
    store::all_time_freed()
}

#[tauri::command]
fn load_presets() -> Result<serde_json::Value, String> {
    store::load_presets()
}

#[tauri::command]
fn save_presets(presets: serde_json::Value) -> Result<(), String> {
    store::save_presets(&presets)
}

fn write_log(lines: &[String]) -> String {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Crumbtrail")
        .join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!(
        "crumbtrail-{}.log",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    let _ = std::fs::write(&path, lines.join("\n"));
    path.display().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(SpaceState::default())
        .manage(UpdateState::default())
        .invoke_handler(tauri::generate_handler![
            get_categories,
            elevated,
            scan,
            clean,
            drives,
            space_scan,
            space_children,
            space_diff,
            snapshots,
            reveal,
            open_launcher,
            launcher_icons,
            relaunch_admin,
            all_time_freed,
            load_presets,
            save_presets,
            update_check,
            update_download,
            update_apply,
            weekly_status,
            set_weekly
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Err(e) = updater::apply_on_exit() {
                    write_log(&[format!("update not applied on exit: {e}")]);
                }
            }
        });
}
