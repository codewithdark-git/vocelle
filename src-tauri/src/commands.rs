//! Commands the Settings window calls (speech-model commands live in `speech.rs`).

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    asr::Engine,
    audio::{self, Recording},
    cleanup::{self, Replacement},
    dictation,
    history::{Entry, History, Stats},
    hotkey, insert,
    keys::TalkKey,
    models,
    settings::{Settings, Store, LANGUAGES},
};

#[derive(Serialize)]
pub struct AppInfo {
    name: String,
    version: String,
    platform: &'static str,
    /// A Wayland session, where the talk key can't be heard yet.
    wayland: bool,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let pkg = app.package_info();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        platform: std::env::consts::OS,
        wayland: wayland_session(),
    }
}

/// Whether this is a Linux Wayland session. The talk key listens through
/// X11, which Wayland apps don't share key presses with.
pub fn wayland_session() -> bool {
    let var = |name| std::env::var(name).ok();
    cfg!(target_os = "linux")
        && is_wayland(
            var("XDG_SESSION_TYPE").as_deref(),
            var("WAYLAND_DISPLAY").as_deref(),
        )
}

/// XDG_SESSION_TYPE decides when it's set; otherwise a WAYLAND_DISPLAY means
/// Wayland.
fn is_wayland(session_type: Option<&str>, wayland_display: Option<&str>) -> bool {
    match session_type.map(str::trim) {
        Some(kind) if kind.eq_ignore_ascii_case("wayland") => true,
        Some(kind) if !kind.is_empty() && !kind.eq_ignore_ascii_case("tty") => false,
        _ => wayland_display.is_some_and(|d| !d.trim().is_empty()),
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

/// Async, so listing microphones runs off the main thread.
#[tauri::command]
pub async fn settings_choices() -> Choices {
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

/// Runs text cleanup on sample text so the Text page can preview a level,
/// with the rules for the language set in Settings.
#[tauri::command]
pub fn preview_cleanup(
    engine: State<'_, Engine>,
    text: String,
    level: cleanup::Level,
    language: String,
    replacements: Vec<Replacement>,
) -> String {
    let english_only = engine
        .loaded_model()
        .and_then(models::find)
        .is_some_and(|m| m.english_only);
    cleanup::clean(
        &text,
        level,
        preview_language(&language, english_only),
        &replacements,
    )
}

/// The language a preview is cleaned as. With "auto", Whisper picks the
/// language from the speech; the sample is typed, not spoken, and it's
/// English, so it's previewed as English.
fn preview_language(setting: &str, english_only: bool) -> &str {
    match setting {
        _ if english_only => "en",
        "" | "auto" => "en",
        code => code,
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

/// Async, so reading the history file runs off the main thread.
#[tauri::command]
pub async fn list_history(app: AppHandle) -> Vec<Entry> {
    app.state::<History>().list(&app)
}

#[tauri::command]
pub fn delete_history_entry(app: AppHandle, history: State<'_, History>, at: u64) {
    history.delete(&app, at);
}

#[tauri::command]
pub fn clear_history(
    app: AppHandle,
    history: State<'_, History>,
    shared: State<'_, dictation::Shared>,
) {
    history.clear(&app);
    // "Paste last dictation" shouldn't bring back text the user just cleared.
    shared.forget();
}

/// Pastes the last dictation, or copies it when Vocelle's own window has focus
/// (as it does when this is called from the Settings window).
#[tauri::command]
pub fn paste_last(app: AppHandle) {
    dictation::paste_last(&app);
}

/// Off the main thread: it can wait a moment for the last paste.
#[tauri::command(async)]
pub fn copy_text(text: String) -> Result<(), String> {
    insert::copy_for_user(&text).map_err(|e| format!("Couldn't copy: {e}"))
}

/// The studio's support page, opened at its contribution section.
const SUPPORT_URL: &str = "https://xcollab.pages.dev/support#contribute";

/// Opens the support page in the default browser. The address is fixed here,
/// so the window can't use this to open anything else.
#[tauri::command]
pub fn open_support_page() -> Result<(), String> {
    #[cfg(windows)]
    let result = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", SUPPORT_URL])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    };
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(SUPPORT_URL).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open")
        .arg(SUPPORT_URL)
        .spawn();

    result
        .map(|_| ())
        .map_err(|e| format!("Couldn't open your browser ({e}). The support page is {SUPPORT_URL}"))
}

#[tauri::command]
pub fn get_stats(app: AppHandle, history: State<'_, History>) -> Stats {
    history.stats(&app)
}

/// Why the talk key isn't working (its listener couldn't start), or `None`.
#[tauri::command]
pub fn talk_key_problem() -> Option<String> {
    hotkey::failure()
}

/// Listens to `microphone` for two seconds and returns the loudest level heard
/// (0.0–1.0), so the welcome setup can show whether the microphone works.
#[tauri::command(async)]
pub fn test_microphone(microphone: Option<String>) -> Result<f32, String> {
    let rec = Recording::start(microphone.as_deref())?;
    let mut peak = 0.0_f32;
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(50));
        peak = peak.max(rec.level());
    }
    Ok(peak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_uses_the_language_setting() {
        assert_eq!(preview_language("de", false), "de");
        assert_eq!(preview_language("auto", false), "en");
        assert_eq!(preview_language("de", true), "en");
    }

    #[test]
    fn wayland_is_detected_from_the_session() {
        assert!(is_wayland(Some("wayland"), None));
        assert!(is_wayland(Some("Wayland"), Some("wayland-0")));
        assert!(!is_wayland(Some("x11"), None));
        // An X11 session that happens to run a nested Wayland compositor.
        assert!(!is_wayland(Some("x11"), Some("wayland-1")));
        assert!(is_wayland(None, Some("wayland-0")));
        assert!(is_wayland(Some("tty"), Some("wayland-0")));
        assert!(!is_wayland(None, Some("")));
        assert!(!is_wayland(None, None));
    }
}
