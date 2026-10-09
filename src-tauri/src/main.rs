// No console window, in release or debug. A debug build (the dev app, and its "Restart as admin"
// relaunch) used to flash a command prompt. Piped stdout/stderr from `tauri dev` still work. DO NOT REMOVE!!
#![windows_subsystem = "windows"]

fn main() {
    // Must run first: handles install/update/uninstall hooks and applies a
    // downloaded update before the app starts.
    velopack::VelopackApp::build()
        .on_before_uninstall_fast_callback(|_| crumbtrail_lib::remove_weekly_task())
        .run();

    if std::env::args().any(|a| a == "--snapshot") {
        crumbtrail_lib::run_snapshot_task();
        return;
    }
    crumbtrail_lib::run()
}
