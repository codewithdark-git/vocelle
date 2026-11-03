//! A small log file next to the user's data, so slow or failed dictations can
//! be diagnosed on real machines. Lines are timestamped and the file is
//! capped: when it passes `MAX_BYTES` it's renamed to `vocelle.old.log` and a
//! new one is started. Only timings and status are logged, never dictated text.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
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
    let _lock = LOCK.lock().unwrap();
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

/// `log!("format {args}")`: writes a formatted line to the log.
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}
pub(crate) use log;
