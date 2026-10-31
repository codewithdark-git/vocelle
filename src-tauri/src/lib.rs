mod asr;
mod audio;
mod dictation;
mod hotkey;
mod insert;
mod overlay;
mod tray;

use serde::Serialize;
use tauri::{Manager, WindowEvent};

#[derive(Serialize)]
struct AppInfo {
    name: String,
    version: String,
}

#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfo {
    let pkg = app.package_info();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
    }
}

/// Brings the settings window to the front, restoring it if hidden or minimized.
pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        // Must be registered first so a second launch exits before doing any setup.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .setup(|app| {
            tray::create(app.handle())?;
            overlay::init(app.handle());
            dictation::start(app.handle().clone())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window hides it; the app keeps running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("error while running Vocelle");
}
