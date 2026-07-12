mod categories;
mod engine;

use categories::{build_categories, CategoryInfo};
use engine::{clean_category, is_elevated, scan_category, CleanResult};
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn get_categories() -> Vec<CategoryInfo> {
    build_categories().iter().map(|c| c.info()).collect()
}

#[tauri::command]
fn elevated() -> bool {
    is_elevated()
}

#[tauri::command]
fn scan(app: AppHandle) {
    std::thread::spawn(move || {
        let cats = build_categories();
        let mut handles = Vec::new();
        for cat in cats {
            let app2 = app.clone();
            handles.push(std::thread::spawn(move || {
                let res = scan_category(&cat);
                let _ = app2.emit("scan:result", &res);
            }));
        }
        for h in handles {
            let _ = h.join();
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
        log.push(format!("Sweep run {}", chrono::Local::now().to_rfc3339()));
        let mut total = 0u64;
        for cat in cats.iter().filter(|c| ids.iter().any(|id| id == c.id)) {
            if cat.needs_admin && !admin {
                log.push(format!("[{}] skipped — needs admin", cat.id));
                let _ = app.emit(
                    "clean:result",
                    &CleanResult { id: cat.id, freed_bytes: 0, deleted: 0, skipped: 0 },
                );
                continue;
            }
            let res = clean_category(cat, &mut log);
            total += res.freed_bytes;
            let _ = app.emit("clean:result", &res);
        }
        let log_path = write_log(&log);
        let _ = app.emit(
            "clean:done",
            serde_json::json!({ "total_bytes": total, "log_path": log_path }),
        );
    });
}

#[tauri::command]
fn relaunch_admin(app: AppHandle) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display()),
            ])
            .spawn();
        app.exit(0);
    }
}

fn write_log(lines: &[String]) -> String {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Sweep")
        .join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!(
        "sweep-{}.log",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    let _ = std::fs::write(&path, lines.join("\n"));
    path.display().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_categories,
            elevated,
            scan,
            clean,
            relaunch_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
