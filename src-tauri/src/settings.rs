//! User settings, stored as JSON in the app's config folder and kept in memory.
//!
//! Every field has a default, so settings files from older versions keep
//! loading and new fields get sensible values.

use std::{fs, path::PathBuf, sync::RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{
    cleanup::{self, Replacement},
    keys,
};

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
pub struct Store(RwLock<Settings>);

impl Store {
    pub fn load(app: &AppHandle) -> Self {
        let settings: Settings = file(app)
            .and_then(|f| fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // A hand-edited file with a bad value falls back to defaults for that file.
        Self(RwLock::new(settings.validated().unwrap_or_default()))
    }

    pub fn get(&self) -> Settings {
        self.0.read().unwrap().clone()
    }

    /// Validates, saves to disk, then replaces the in-memory copy.
    pub fn set(&self, app: &AppHandle, settings: Settings) -> Result<Settings, String> {
        let settings = settings.validated()?;
        let path = file(app).ok_or("No settings folder available")?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("Couldn't save settings: {e}"))?;
        *self.0.write().unwrap() = settings.clone();
        Ok(settings)
    }

    /// Applies a change to the current settings and saves it.
    pub fn update(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Settings),
    ) -> Result<Settings, String> {
        let mut settings = self.get();
        change(&mut settings);
        self.set(app, settings)
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
}
