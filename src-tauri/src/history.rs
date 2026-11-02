//! Recent dictations, kept in a small JSON file in the app's data folder.
//!
//! Only the text is stored (never audio), and the user can turn history off
//! or clear it at any time.

use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Oldest entries are dropped beyond this.
const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Milliseconds since the Unix epoch; doubles as the id.
    pub at: u64,
    pub text: String,
    /// Length of the recording.
    pub seconds: f32,
    pub model: String,
}

/// Serializes file access between the dictation thread and Settings commands.
#[derive(Default)]
pub struct History(Mutex<()>);

impl History {
    pub fn list(&self, app: &AppHandle) -> Vec<Entry> {
        let _lock = self.0.lock().unwrap();
        read(app)
    }

    pub fn add(&self, app: &AppHandle, text: &str, seconds: f32, model: &str) {
        let _lock = self.0.lock().unwrap();
        let mut entries = read(app);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        // Two dictations in the same millisecond would share an id.
        let at = entries.first().map_or(now, |e| now.max(e.at + 1));
        entries.insert(
            0,
            Entry {
                at,
                text: text.to_owned(),
                seconds,
                model: model.to_owned(),
            },
        );
        entries.truncate(MAX_ENTRIES);
        write(app, &entries);
    }

    pub fn delete(&self, app: &AppHandle, at: u64) {
        let _lock = self.0.lock().unwrap();
        let mut entries = read(app);
        entries.retain(|e| e.at != at);
        write(app, &entries);
    }

    pub fn clear(&self, app: &AppHandle) {
        let _lock = self.0.lock().unwrap();
        if let Some(path) = file(app) {
            let _ = fs::remove_file(path);
        }
    }
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|d| d.join("history.json"))
}

fn read(app: &AppHandle) -> Vec<Entry> {
    file(app)
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write(app: &AppHandle, entries: &[Entry]) {
    let Some(path) = file(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    match serde_json::to_string(entries) {
        Ok(json) => {
            if let Err(err) = fs::write(&path, json) {
                eprintln!("vocelle: couldn't save history: {err}");
            }
        }
        Err(err) => eprintln!("vocelle: couldn't save history: {err}"),
    }
}
