//! Recent dictations, kept in a small JSON file in the app's data folder,
//! and running totals for the Home page.
//!
//! Only the text is stored (never audio), and the user can turn history off
//! or clear it at any time. The totals are just counts, kept even when
//! history is off.

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

/// Totals across all dictations, for the Home page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub dictations: u64,
    pub words: u64,
    /// Total length of the recordings.
    pub seconds: f64,
    /// Transcription time per second of audio for the last few dictations,
    /// newest last. Home suggests a faster model when these are high.
    pub recent_speed: Vec<f32>,
}

/// How many recent dictations `recent_speed` keeps.
const RECENT: usize = 5;

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

    pub fn stats(&self, app: &AppHandle) -> Stats {
        let _lock = self.0.lock().unwrap();
        read_json(&data_file(app, "stats.json")).unwrap_or_default()
    }

    /// Adds one dictation to the totals. `speed` is transcription time per
    /// second of audio.
    pub fn count(&self, app: &AppHandle, text: &str, seconds: f32, speed: f32) {
        let _lock = self.0.lock().unwrap();
        let path = data_file(app, "stats.json");
        let mut stats: Stats = read_json(&path).unwrap_or_default();
        stats.dictations += 1;
        stats.words += text.split_whitespace().count() as u64;
        stats.seconds += f64::from(seconds);
        stats.recent_speed.push(speed);
        let excess = stats.recent_speed.len().saturating_sub(RECENT);
        stats.recent_speed.drain(..excess);
        write_json(&path, &stats);
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
    data_file(app, "history.json")
}

fn data_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join(name))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Option<PathBuf>) -> Option<T> {
    let text = fs::read_to_string(path.as_ref()?).ok()?;
    serde_json::from_str(&text).ok()
}

fn read(app: &AppHandle) -> Vec<Entry> {
    read_json(&file(app)).unwrap_or_default()
}

fn write(app: &AppHandle, entries: &[Entry]) {
    write_json(&file(app), entries);
}

fn write_json<T: Serialize + ?Sized>(path: &Option<PathBuf>, value: &T) {
    let Some(path) = path else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    match serde_json::to_string(value) {
        Ok(json) => {
            if let Err(err) = fs::write(path, json) {
                eprintln!("vocelle: couldn't save history: {err}");
            }
        }
        Err(err) => eprintln!("vocelle: couldn't save history: {err}"),
    }
}
