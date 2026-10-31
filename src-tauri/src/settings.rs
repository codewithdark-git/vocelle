//! User settings, stored as JSON in the app's config folder.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Catalog id of the speech model to use. `None` until the user picks or downloads one.
    pub model: Option<String>,
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    file(app)
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = file(app).ok_or("No settings folder available")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Couldn't save settings: {e}"))
}
