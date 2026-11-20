//! A small log file next to the user's data, so slow or failed dictations can
//! be diagnosed on real machines. Lines are timestamped and the file is
//! capped: when it passes `MAX_BYTES` it's renamed to `vocelle.old.log` and a
//! new one is started. Only timings and status are logged, never dictated text.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: u64 = 256 * 1024;

static FILE: OnceLock<PathBuf> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Sets the log folder. Until this is called, messages only go to stderr.
pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let _ = FILE.set(dir.join("vocelle.log"));
}

pub fn write(message: &str) {
    eprintln!("vocelle: {message}");
    let Some(path) = FILE.get() else { return };
    // A panic elsewhere while this was held mustn't stop logging for good.
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = fs::rename(path, path.with_file_name("vocelle.old.log"));
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{secs} {message}");
    }
}

/// Logs panics before the default hook runs. Release builds use
/// `panic = "abort"`, so the process ends right after this: the line is
/// written straight to the file and synced, and the log lock is only tried,
/// never waited on (the panic may have happened while it was held). A crash
/// marker is left too, so the next start can tell the user.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let what = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown cause");
        let thread = std::thread::current();
        let line = panic_line(
            thread.name().unwrap_or("unnamed"),
            what,
            info.location().map(|l| (l.file(), l.line())),
        );
        write_crash(&line);
        default(info);
    }));
}

fn panic_line(thread: &str, what: &str, location: Option<(&str, u32)>) -> String {
    match location {
        Some((file, line)) => format!("panic in thread '{thread}' at {file}:{line}: {what}"),
        None => format!("panic in thread '{thread}': {what}"),
    }
}

/// Stderr is left to the default hook, which runs right after.
fn write_crash(line: &str) {
    let Some(path) = FILE.get() else { return };
    let _lock = LOCK.try_lock();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(format!("{secs} {line}\n").as_bytes());
        let _ = file.sync_all();
    }
    let _ = fs::write(crash_marker(path), line);
}

fn crash_marker(log: &Path) -> PathBuf {
    log.with_file_name("vocelle.crash")
}

/// The panic line left by the last run if it crashed, removing the marker so
/// it's reported once.
pub fn take_crash_marker() -> Option<String> {
    let marker = crash_marker(FILE.get()?);
    let line = fs::read_to_string(&marker).ok()?;
    let _ = fs::remove_file(&marker);
    Some(line)
}

/// `log!("format {args}")`: writes a formatted line to the log.
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}
pub(crate) use log;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_line_names_thread_place_and_cause() {
        assert_eq!(
            panic_line("dictation", "boom", Some(("src/x.rs", 7))),
            "panic in thread 'dictation' at src/x.rs:7: boom"
        );
        assert_eq!(
            panic_line("main", "boom", None),
            "panic in thread 'main': boom"
        );
    }
}
