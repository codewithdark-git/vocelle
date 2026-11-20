//! User settings, stored as JSON in the app's config folder and kept in memory.
//!
//! Every field has a default, so settings files from older versions keep
//! loading and new fields get sensible values.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, RwLock,
    },
    time::Duration,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};

use crate::{
    cleanup::{self, Replacement},
    keys,
    log::log,
    problem, store_io,
};

/// Tries at reading a settings file that exists but can't be read (a virus
/// scanner or sync tool holding it, say), a short pause apart.
const READ_TRIES: u32 = 5;
const READ_RETRY: Duration = Duration::from_millis(200);

/// Shown on Home when the settings file couldn't be read at start-up.
const UNREAD_MESSAGE: &str =
    "Couldn't read your settings file, so Vocelle is using default settings \
     for now. Restart Vocelle to try again; your saved settings are kept.";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Unknown keys are ignored, so files from 0.2 with "talk_mode" still load.
#[serde(default)]
pub struct Settings {
    /// Catalog id of the speech model to use. `None` until the user picks or downloads one.
    pub model: Option<String>,
    /// `KeyboardEvent.code` of the talk key, e.g. "ControlRight".
    pub talk_key: String,
    /// Input device name; `None` uses the system default microphone.
    pub microphone: Option<String>,
    /// "auto" or a language code such as "en" (ignored by English-only models).
    pub language: String,
    pub cleanup: cleanup::Level,
    /// Adds a space after each dictation so consecutive ones don't run together.
    pub trailing_space: bool,
    /// Names and terms Whisper should spell correctly.
    pub dictionary: Vec<String>,
    pub replacements: Vec<Replacement>,
    pub history: bool,
    /// Whether the welcome setup has been completed or skipped.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: None,
            talk_key: keys::DEFAULT.to_owned(),
            microphone: None,
            language: "auto".to_owned(),
            cleanup: cleanup::Level::default(),
            trailing_space: true,
            dictionary: Vec::new(),
            replacements: Vec::new(),
            history: true,
            onboarded: false,
        }
    }
}

/// Languages offered for multilingual models. Whisper supports more; these
/// cover most users and keep the picker short.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("auto", "Detect automatically"),
    ("en", "English"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("it", "Italian"),
    ("pt", "Portuguese"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("tr", "Turkish"),
    ("ru", "Russian"),
    ("uk", "Ukrainian"),
    ("ar", "Arabic"),
    ("hi", "Hindi"),
    ("ur", "Urdu"),
    ("bn", "Bengali"),
    ("zh", "Chinese"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("id", "Indonesian"),
    ("vi", "Vietnamese"),
];

impl Settings {
    /// Rejects values the app can't act on, and trims the dictionary.
    pub fn validated(mut self) -> Result<Self, String> {
        if keys::find(&self.talk_key).is_none() {
            return Err(format!("{} can't be used as the talk key", self.talk_key));
        }
        if !LANGUAGES.iter().any(|(code, _)| *code == self.language) {
            return Err(format!("Unsupported language: {}", self.language));
        }
        self.dictionary = self
            .dictionary
            .iter()
            .map(|w| w.trim().to_owned())
            .filter(|w| !w.is_empty())
            .collect();
        self.replacements.retain(|r| !r.from.trim().is_empty());
        self.microphone = self.microphone.filter(|m| !m.trim().is_empty());
        Ok(self)
    }
}

/// In-memory settings shared across threads.
pub struct Store {
    settings: RwLock<Settings>,
    /// The settings file exists but couldn't be read. Defaults are in use
    /// until it can be, and saving would replace the user's settings with
    /// them, so every save reads the file first.
    unread: AtomicBool,
    /// Held from reading the settings to saving them, so two saves (the
    /// Settings window and a model download, say) can't undo each other.
    saving: Mutex<()>,
}

impl Store {
    pub fn load(app: &AppHandle) -> Self {
        let store = match file(app) {
            Some(path) => Self::load_from(&path),
            None => Self::new(Settings::default(), false),
        };
        if store.unread.load(Ordering::Relaxed) {
            problem::report(app, UNREAD_MESSAGE);
        }
        store
    }

    /// A damaged file is moved aside and its backup used. A file that can't
    /// be read at all is tried again briefly, then left alone: defaults are
    /// used, and it isn't overwritten until it has been read.
    fn load_from(path: &Path) -> Self {
        let mut tries = 0;
        loop {
            tries += 1;
            match store_io::read_or_recover(path, parse) {
                Ok(settings) => return Self::new(settings.unwrap_or_default(), false),
                Err(_) if tries < READ_TRIES => std::thread::sleep(READ_RETRY),
                Err(_) => return Self::new(Settings::default(), true),
            }
        }
    }

    fn new(mut settings: Settings, unread: bool) -> Self {
        if unread {
            // Not a first run: don't offer the welcome setup again.
            settings.onboarded = true;
        }
        Self {
            settings: RwLock::new(settings),
            unread: AtomicBool::new(unread),
            saving: Mutex::new(()),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    /// Replaces the settings with the ones the Settings window shows, and
    /// saves them. The model is managed by the Models page, so it's kept.
    pub fn set(&self, app: &AppHandle, settings: Settings) -> Result<Settings, String> {
        let path = file(app).ok_or("No settings folder available")?;
        self.save(&path, |current, reloaded| {
            if reloaded {
                // The window was showing defaults, not these settings.
                return Err(
                    "Your saved settings could be read now and are back, so this \
                            change wasn't saved. Restart Vocelle and make it again."
                        .to_owned(),
                );
            }
            *current = Settings {
                model: current.model.take(),
                ..settings
            };
            Ok(())
        })
    }

    /// Applies a change to the current settings and saves it.
    pub fn update(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Settings),
    ) -> Result<Settings, String> {
        let path = file(app).ok_or("No settings folder available")?;
        self.save(&path, |current, _| {
            change(current);
            Ok(())
        })
    }

    /// Applies `change` to the latest settings, validates, saves to disk,
    /// then replaces the in-memory copy. `change` is told whether the file
    /// was only just read, replacing the defaults in use until then.
    fn save(
        &self,
        path: &Path,
        change: impl FnOnce(&mut Settings, bool) -> Result<(), String>,
    ) -> Result<Settings, String> {
        let _saving = self.saving.lock().unwrap_or_else(|e| e.into_inner());
        let reloaded = self.read_unread(path)?;
        let mut settings = self.get();
        change(&mut settings, reloaded)?;
        let settings = settings.validated()?;
        let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        store_io::atomic_write(path, json.as_bytes()).map_err(|e| {
            log!("couldn't save settings: {e}");
            format!("Couldn't save settings: {e}")
        })?;
        *self.settings.write().unwrap() = settings.clone();
        Ok(settings)
    }

    /// If the file couldn't be read at start-up, reads it now. Returns
    /// whether it was read and replaced the defaults; fails, so nothing is
    /// saved over it, while it still can't be read.
    fn read_unread(&self, path: &Path) -> Result<bool, String> {
        if !self.unread.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let found = store_io::read_or_recover(path, parse).map_err(|e| {
            format!(
                "Couldn't read your settings file, so nothing was saved, to keep it from \
                 being overwritten. Restart Vocelle to try again. Reason: {e}"
            )
        })?;
        log!("the settings file could be read after all");
        let reloaded = found.is_some();
        if let Some(settings) = found {
            *self.settings.write().unwrap() = settings;
        }
        self.unread.store(false, Ordering::Relaxed);
        Ok(reloaded)
    }
}

/// Parses a settings file one field at a time: a field with a bad value
/// (wrong type, unknown talk key or language) gets its default and the
/// others are kept, so one hand edit can't wipe the dictionary. Only a file
/// that isn't a JSON object at all is an error.
fn parse(text: &str) -> Result<Settings, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let Value::Object(map) = value else {
        return Err("not a JSON object".to_owned());
    };
    let mut settings = Settings::default();
    let mut bad: Vec<&str> = Vec::new();
    // Listing every field here (no `..`) makes the compiler flag new ones.
    let Settings {
        model,
        talk_key,
        microphone,
        language,
        cleanup,
        trailing_space,
        dictionary,
        replacements,
        history,
        onboarded,
    } = &mut settings;
    field(&map, "model", model, &mut bad);
    field(&map, "talk_key", talk_key, &mut bad);
    field(&map, "microphone", microphone, &mut bad);
    field(&map, "language", language, &mut bad);
    field(&map, "cleanup", cleanup, &mut bad);
    field(&map, "trailing_space", trailing_space, &mut bad);
    list_field(&map, "dictionary", dictionary, &mut bad);
    list_field(&map, "replacements", replacements, &mut bad);
    field(&map, "history", history, &mut bad);
    field(&map, "onboarded", onboarded, &mut bad);

    let defaults = Settings::default();
    if keys::find(&settings.talk_key).is_none() {
        settings.talk_key = defaults.talk_key;
        bad.push("talk_key");
    }
    if !LANGUAGES.iter().any(|(code, _)| *code == settings.language) {
        settings.language = defaults.language;
        bad.push("language");
    }
    if !bad.is_empty() {
        // Field names only: values may hold personal words.
        log!("settings: reset invalid {} to defaults", bad.join(", "));
    }
    // Only trims and drops empty entries now that the checks above passed.
    settings.validated()
}

/// Sets `target` from `map[key]` if present and valid; otherwise leaves the
/// default and notes the key.
fn field<T: DeserializeOwned>(
    map: &Map<String, Value>,
    key: &'static str,
    target: &mut T,
    bad: &mut Vec<&'static str>,
) {
    if let Some(value) = map.get(key) {
        match T::deserialize(value) {
            Ok(v) => *target = v,
            Err(_) => bad.push(key),
        }
    }
}

/// Like `field` for lists, but keeps the valid items when only some are bad.
fn list_field<T: DeserializeOwned>(
    map: &Map<String, Value>,
    key: &'static str,
    target: &mut Vec<T>,
    bad: &mut Vec<&'static str>,
) {
    let Some(value) = map.get(key) else { return };
    let Some(items) = value.as_array() else {
        bad.push(key);
        return;
    };
    *target = items
        .iter()
        .filter_map(|item| T::deserialize(item).ok())
        .collect();
    if target.len() < items.len() {
        bad.push(key);
    }
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_files_still_load() {
        let s: Settings = serde_json::from_str(r#"{ "model": "base.en" }"#).unwrap();
        assert_eq!(s.model.as_deref(), Some("base.en"));
        assert_eq!(s.talk_key, "ControlRight");
        assert!(s.trailing_space);
        assert!(s.history);
    }

    #[test]
    fn toggle_mode_from_0_2_falls_back_to_hold() {
        let s: Settings =
            serde_json::from_str(r#"{ "talk_key": "F9", "talk_mode": "toggle" }"#).unwrap();
        assert_eq!(s.talk_key, "F9");
    }

    #[test]
    fn validation_rejects_unknown_keys_and_languages() {
        let bad_key = Settings {
            talk_key: "KeyA".into(),
            ..Settings::default()
        };
        assert!(bad_key.validated().is_err());
        let bad_lang = Settings {
            language: "xx".into(),
            ..Settings::default()
        };
        assert!(bad_lang.validated().is_err());
    }

    #[test]
    fn validation_trims_dictionary_and_drops_empty_rules() {
        let s = Settings {
            dictionary: vec!["  Vocelle ".into(), "".into()],
            replacements: vec![
                Replacement {
                    from: " ".into(),
                    to: "x".into(),
                },
                Replacement {
                    from: "pr".into(),
                    to: "PR".into(),
                },
            ],
            microphone: Some("  ".into()),
            ..Settings::default()
        }
        .validated()
        .unwrap();
        assert_eq!(s.dictionary, vec!["Vocelle"]);
        assert_eq!(s.replacements.len(), 1);
        assert_eq!(s.microphone, None);
    }
    #[test]
    fn one_invalid_field_keeps_the_others() {
        let s = parse(
            r#"{
                "model": "base.en",
                "cleanup": "extreme",
                "trailing_space": "yes",
                "dictionary": ["Vocelle", 5, "Tauri"],
                "replacements": [{ "from": "pr", "to": "PR" }, { "from": 1 }],
                "history": false
            }"#,
        )
        .unwrap();
        assert_eq!(s.model.as_deref(), Some("base.en"));
        assert_eq!(s.cleanup, cleanup::Level::default());
        assert!(s.trailing_space);
        assert_eq!(s.dictionary, vec!["Vocelle", "Tauri"]);
        assert_eq!(s.replacements.len(), 1);
        assert!(!s.history);
    }

    #[test]
    fn unknown_talk_key_or_language_falls_back_per_field() {
        let s = parse(r#"{ "talk_key": "KeyA", "language": "xx", "dictionary": ["Vocelle"] }"#)
            .unwrap();
        assert_eq!(s.talk_key, keys::DEFAULT);
        assert_eq!(s.language, "auto");
        assert_eq!(s.dictionary, vec!["Vocelle"]);
    }

    #[test]
    fn parse_round_trips_every_field() {
        let s = Settings {
            model: Some("small".into()),
            talk_key: "F9".into(),
            microphone: Some("USB Mic".into()),
            language: "de".into(),
            cleanup: cleanup::Level::Full,
            trailing_space: false,
            dictionary: vec!["Vocelle".into()],
            replacements: vec![Replacement {
                from: "pr".into(),
                to: "PR".into(),
            }],
            history: false,
            onboarded: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse(&json).unwrap(), s);
        assert!(parse("[1, 2]").is_err());
        assert!(parse("{ \"model\": ").is_err());
    }

    #[test]
    fn saves_never_overwrite_a_file_that_was_not_read() {
        let dir = store_io::test_dir("settings-unread");
        let path = dir.join("settings.json");
        let saved = Settings {
            model: Some("base.en".into()),
            dictionary: vec!["Vocelle".into()],
            onboarded: true,
            ..Settings::default()
        };
        let json = serde_json::to_string_pretty(&saved).unwrap();
        std::fs::write(&path, &json).unwrap();
        // As if the file couldn't be read at start-up.
        let store = Store::new(Settings::default(), true);
        assert!(
            store.get().onboarded,
            "no welcome setup for a returning user"
        );
        assert_eq!(store.get().model, None);

        // A save from the window, which was showing defaults, is refused.
        let refused = store.save(&path, |current, reloaded| {
            assert!(reloaded);
            *current = Settings::default();
            Err("stale".into())
        });
        assert!(refused.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
        // The real settings are in use now, and later changes apply to them.
        assert_eq!(store.get(), saved);
        let changed = store
            .save(&path, |current, reloaded| {
                assert!(!reloaded);
                current.trailing_space = false;
                Ok(())
            })
            .unwrap();
        assert_eq!(changed.dictionary, saved.dictionary);
        assert_eq!(changed.model, saved.model);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_fails_while_the_file_still_cannot_be_read() {
        let dir = store_io::test_dir("settings-locked");
        // A folder where the file should be: it exists but can't be read.
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&path).unwrap();
        let store = Store::new(Settings::default(), true);
        let result = store.save(&path, |current, _| {
            current.trailing_space = false;
            Ok(())
        });
        assert!(result
            .unwrap_err()
            .contains("Couldn't read your settings file"));
        assert!(path.is_dir());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_settings_file_recovers_from_backup() {
        let dir = store_io::test_dir("settings");
        let path = dir.join("settings.json");
        let saved = Settings {
            dictionary: vec!["Vocelle".into()],
            ..Settings::default()
        };
        let json = serde_json::to_string_pretty(&saved).unwrap();
        store_io::atomic_write(&path, json.as_bytes()).unwrap();
        store_io::atomic_write(&path, json.as_bytes()).unwrap();
        std::fs::write(&path, &json[..json.len() / 2]).unwrap();
        let loaded = store_io::read_or_recover(&path, parse).unwrap();
        assert_eq!(loaded, Some(saved));
        let _ = std::fs::remove_dir_all(dir);
    }
}
