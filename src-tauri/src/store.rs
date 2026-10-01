// Small per-user settings in %APPDATA%\Crumbtrail: saved presets and the
// all-time freed counter. Written via temp file + rename so a crash can't
// leave half a file behind.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Crumbtrail")
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("couldn't create {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("couldn't write {}: {e}", path.display()))
}

// Missing file = defaults. A damaged file is an error, so we never silently overwrite it.
fn read_json(path: &Path, default: Value) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            serde_json::from_str(&text).map_err(|e| format!("{} is damaged: {e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(default),
        Err(e) => Err(format!("couldn't read {}: {e}", path.display())),
    }
}

pub fn presets_path() -> PathBuf {
    dir().join("presets.json")
}

pub fn load_presets() -> Result<Value, String> {
    read_json(&presets_path(), json!({ "version": 1, "presets": [] }))
}

pub fn save_presets(v: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    write_atomic(&presets_path(), &text)
}

fn stats_path() -> PathBuf {
    dir().join("stats.json")
}

pub fn all_time_freed() -> u64 {
    read_json(&stats_path(), json!({}))
        .ok()
        .and_then(|v| v.get("all_time_freed").and_then(Value::as_u64))
        .unwrap_or(0)
}

/// Add to the running total and return the new total.
pub fn add_freed(bytes: u64) -> Result<u64, String> {
    let mut v = read_json(&stats_path(), json!({}))?;
    let total = v.get("all_time_freed").and_then(Value::as_u64).unwrap_or(0) + bytes;
    v["all_time_freed"] = json!(total);
    write_atomic(&stats_path(), &v.to_string())?;
    Ok(total)
}
