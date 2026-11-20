//! Recent dictations, kept in a small JSON file in the app's data folder,
//! and running totals for the Home page.
//!
//! Only the text is stored (never audio), and the user can turn history off
//! or clear it at any time. The totals are just counts, kept even when
//! history is off.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Sender},
        Mutex, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{log::log, store_io};

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

/// A finished dictation, for the totals and (when history is on) the list.
pub struct Record {
    pub text: String,
    /// Length of the recording.
    pub seconds: f32,
    /// Transcription time per second of audio.
    pub speed: f32,
    /// The model that heard it, or `None` when history is off.
    pub model: Option<String>,
}

#[derive(Default)]
pub struct History {
    /// Serializes file access between the history writer and Settings commands.
    files: Mutex<()>,
    /// Counts "Clear history", so a dictation queued before a clear isn't
    /// written back after it.
    cleared: AtomicU64,
    /// Feeds the background writer, started with the first dictation.
    writer: OnceLock<Sender<(Record, u64)>>,
}

impl History {
    /// Saves a dictation on a background thread, so the text is typed
    /// without waiting for the disk. Writes start at once and keep their order.
    pub fn record(&self, app: &AppHandle, record: Record) {
        let cleared = self.cleared.load(Ordering::SeqCst);
        let writer = self.writer.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<(Record, u64)>();
            let app = app.clone();
            let spawned = std::thread::Builder::new()
                .name("history-writer".into())
                .spawn(move || {
                    let history = app.state::<History>();
                    for (record, cleared) in rx {
                        history.write(&app, &record, cleared);
                    }
                });
            if let Err(err) = spawned {
                log!("couldn't start the history writer: {err}");
            }
            tx
        });
        // Without a writer thread, write here rather than lose it.
        if let Err(mpsc::SendError((record, cleared))) = writer.send((record, cleared)) {
            self.write(app, &record, cleared);
        }
    }

    /// Writes one dictation. `cleared` is the clear count when it was queued.
    fn write(&self, app: &AppHandle, record: &Record, cleared: u64) {
        let _lock = self.lock();
        // Cleared since it was queued: only the totals count it.
        let list = file(app).filter(|_| self.cleared.load(Ordering::SeqCst) == cleared);
        if let Some(stats) = data_file(app, "stats.json") {
            save_record(&stats, list.as_deref(), record);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.files.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn list(&self, app: &AppHandle) -> Vec<Entry> {
        let _lock = self.lock();
        file(app)
            .and_then(|path| read_entries(&path).ok())
            .unwrap_or_default()
    }

    pub fn stats(&self, app: &AppHandle) -> Stats {
        let _lock = self.lock();
        data_file(app, "stats.json")
            .and_then(|path| read_stats(&path).ok())
            .unwrap_or_default()
    }

    pub fn delete(&self, app: &AppHandle, at: u64) {
        let _lock = self.lock();
        if let Some(path) = file(app) {
            delete_entry(&path, at);
        }
    }

    pub fn clear(&self, app: &AppHandle) {
        let _lock = self.lock();
        self.cleared.fetch_add(1, Ordering::SeqCst);
        if let Some(path) = file(app) {
            if let Err(err) = store_io::remove_all(&path) {
                log!("couldn't clear history: {err}");
            }
        }
    }
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    data_file(app, "history.json")
}

fn data_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join(name))
}

/// Parses the history file. Entries that don't parse are dropped (and
/// counted in the log) rather than losing the whole list; only a file that
/// isn't a JSON list at all counts as damaged.
fn parse_entries(text: &str) -> Result<Vec<Entry>, String> {
    let values: Vec<serde_json::Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let total = values.len();
    let entries: Vec<Entry> = values
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    if entries.len() < total {
        log!(
            "skipped {} unreadable history entries",
            total - entries.len()
        );
    }
    Ok(entries)
}

fn read_entries(path: &Path) -> io::Result<Vec<Entry>> {
    Ok(store_io::read_or_recover(path, parse_entries)?.unwrap_or_default())
}

fn read_stats(path: &Path) -> io::Result<Stats> {
    Ok(store_io::read_or_recover(path, |text| {
        serde_json::from_str(text).map_err(|e| e.to_string())
    })?
    .unwrap_or_default())
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T, what: &str) -> bool {
    let result = serde_json::to_vec(value)
        .map_err(io::Error::other)
        .and_then(|json| store_io::atomic_write(path, &json));
    if let Err(err) = &result {
        log!("couldn't save {what}: {err}");
    }
    result.is_ok()
}

fn add_entry(path: &Path, text: &str, seconds: f32, model: &str) {
    // If the file exists but can't be read, leave it alone rather than
    // replace the whole history with this one entry.
    let Ok(mut entries) = read_entries(path) else {
        log!("history left unchanged because it couldn't be read");
        return;
    };
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
    write_json(path, &entries, "history");
}

/// Adds a dictation to the totals, and to the list at `list` when history
/// is on (the record has a model).
fn save_record(stats: &Path, list: Option<&Path>, record: &Record) {
    count_dictation(stats, &record.text, record.seconds, record.speed);
    if let (Some(path), Some(model)) = (list, &record.model) {
        add_entry(path, &record.text, record.seconds, model);
    }
}

fn count_dictation(path: &Path, text: &str, seconds: f32, speed: f32) {
    let Ok(mut stats) = read_stats(path) else {
        log!("totals left unchanged because they couldn't be read");
        return;
    };
    stats.dictations += 1;
    stats.words += text.split_whitespace().count() as u64;
    stats.seconds += f64::from(seconds);
    stats.recent_speed.push(speed);
    let excess = stats.recent_speed.len().saturating_sub(RECENT);
    stats.recent_speed.drain(..excess);
    write_json(path, &stats, "totals");
}

fn delete_entry(path: &Path, at: u64) {
    let Ok(mut entries) = read_entries(path) else {
        log!("history left unchanged because it couldn't be read");
        return;
    };
    entries.retain(|e| e.at != at);
    // The backup still holds the deleted entry; don't keep it on disk.
    if write_json(path, &entries, "history") {
        let _ = fs::remove_file(store_io::backup_path(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_delete_round_trip() {
        let dir = store_io::test_dir("history");
        let path = dir.join("history.json");
        add_entry(&path, "first", 1.0, "base.en");
        add_entry(&path, "second", 2.0, "base.en");
        let entries = read_entries(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second");
        assert!(entries[0].at > entries[1].at);

        delete_entry(&path, entries[0].at);
        let left = read_entries(&path).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].text, "first");
        assert!(!store_io::backup_path(&path).exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn truncated_history_recovers_from_backup_instead_of_shrinking() {
        let dir = store_io::test_dir("history-damaged");
        let path = dir.join("history.json");
        for i in 0..MAX_ENTRIES {
            add_entry(&path, &format!("entry {i}"), 1.0, "base.en");
        }
        // A crash mid-write of the old code left half a file.
        let full = fs::read_to_string(&path).unwrap();
        fs::write(&path, &full[..full.len() / 2]).unwrap();

        add_entry(&path, "after crash", 1.0, "base.en");
        let entries = read_entries(&path).unwrap();
        // The backup was one entry behind; it's restored, then the new one added.
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].text, "after crash");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn one_bad_entry_does_not_lose_the_rest() {
        let parsed = parse_entries(
            r#"[{"at":2,"text":"ok","seconds":1.0,"model":"m"},{"at":"x"},
                {"at":1,"text":"also ok","seconds":1.5,"model":"m"}]"#,
        )
        .unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].text, "also ok");
        assert!(parse_entries("{").is_err());
    }

    #[test]
    fn records_go_to_the_list_only_with_history_on() {
        let dir = store_io::test_dir("records");
        let (stats, list) = (dir.join("stats.json"), dir.join("history.json"));
        let record = |text: &str, model: Option<&str>| Record {
            text: text.into(),
            seconds: 1.0,
            speed: 0.2,
            model: model.map(Into::into),
        };
        save_record(&stats, Some(&list), &record("kept", Some("base.en")));
        save_record(&stats, Some(&list), &record("history off", None));
        // Queued before "Clear history": counted, not listed.
        save_record(&stats, None, &record("cleared", Some("base.en")));
        let entries = read_entries(&list).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "kept");
        assert_eq!(read_stats(&stats).unwrap().dictations, 3);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stats_count_and_keep_recent_speeds() {
        let dir = store_io::test_dir("stats");
        let path = dir.join("stats.json");
        for _ in 0..7 {
            count_dictation(&path, "two words", 2.0, 0.5);
        }
        let stats = read_stats(&path).unwrap();
        assert_eq!(stats.dictations, 7);
        assert_eq!(stats.words, 14);
        assert_eq!(stats.recent_speed.len(), RECENT);
        let _ = fs::remove_dir_all(dir);
    }
}
