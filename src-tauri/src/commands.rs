//! Commands the Settings window calls (speech-model commands live in `speech.rs`).

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    audio,
    history::{Entry, History},
    hotkey,
    keys::TalkKey,
    settings::{Settings, Store, LANGUAGES},
};

#[derive(Serialize)]
pub struct AppInfo {
    name: String,
    version: String,
    platform: &'static str,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let pkg = app.package_info();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        platform: std::env::consts::OS,
    }
}

/// Everything the Settings page needs to render its pickers.
#[derive(Serialize)]
pub struct Choices {
    talk_keys: Vec<TalkKey>,
    languages: Vec<Language>,
    microphones: Vec<String>,
}

#[derive(Serialize)]
pub struct Language {
    code: &'static str,
    name: &'static str,
}

#[tauri::command]
pub fn settings_choices() -> Choices {
    Choices {
        talk_keys: hotkey::available_keys(),
        languages: LANGUAGES
            .iter()
            .map(|(code, name)| Language { code, name })
            .collect(),
        microphones: audio::input_devices(),
    }
}

#[tauri::command]
pub fn get_settings(store: State<'_, Store>) -> Settings {
    store.get()
}

/// Saves settings and applies them right away. The model is managed by the
/// Models page, so it's kept as is here.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    store: State<'_, Store>,
    settings: Settings,
) -> Result<Settings, String> {
    let current = store.get();
    let settings = Settings {
        model: current.model,
        ..settings
    };
    if settings.talk_key != current.talk_key {
        hotkey::set_talk_key(&settings.talk_key)?;
    }
    match store.set(&app, settings) {
        Ok(saved) => Ok(saved),
        Err(err) => {
            // Keep the listener in step with what's actually saved.
            let _ = hotkey::set_talk_key(&current.talk_key);
            Err(err)
        }
    }
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| format!("Couldn't change start-up setting: {e}"))
}

#[tauri::command]
pub fn list_history(app: AppHandle, history: State<'_, History>) -> Vec<Entry> {
    history.list(&app)
}

#[tauri::command]
pub fn delete_history_entry(app: AppHandle, history: State<'_, History>, at: u64) {
    history.delete(&app, at);
}

#[tauri::command]
pub fn clear_history(app: AppHandle, history: State<'_, History>) {
    history.clear(&app);
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .map_err(|e| format!("Couldn't copy: {e}"))
}
