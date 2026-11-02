mod asr;
mod audio;
mod cleanup;
mod commands;
mod dictation;
mod history;
mod hotkey;
mod insert;
mod keys;
mod models;
mod overlay;
mod settings;
mod speech;
mod tray;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

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
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(asr::Engine::default())
        .manage(speech::Downloads::default())
        .manage(history::History::default())
        .setup(|app| {
            app.manage(settings::Store::load(app.handle()));
            // The Settings window is created only now, after the state its
            // commands use is registered (it has `"create": false` in the config).
            let main = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .expect("main window is configured");
            tauri::WebviewWindowBuilder::from_config(app.handle(), main)?.build()?;
            tray::create(app.handle())?;
            overlay::init(app.handle());
            dictation::start(app.handle().clone())?;
            speech::load_selected(app.handle());
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
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::settings_choices,
            commands::get_settings,
            commands::save_settings,
            commands::preview_cleanup,
            commands::get_autostart,
            commands::set_autostart,
            commands::list_history,
            commands::delete_history_entry,
            commands::clear_history,
            commands::copy_text,
            speech::list_models,
            speech::download_model,
            speech::select_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vocelle");
}
