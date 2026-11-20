//! Crash-safe reads and writes for the small JSON files the app keeps
//! (settings, history and stats).
//!
//! Writes go to a temporary file next to the target, are flushed to disk and
//! then renamed over the target, so a crash or power loss leaves either the
//! old file or the new one, never half of each. The previous version is kept
//! as `<name>.bak`.
//!
//! When a file can't be parsed it is moved aside as `<name>.corrupt-<unix
//! seconds>` (so nothing is ever silently overwritten) and the backup is
//! tried instead.

use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::log::log;

/// Makes temporary file names unique when two threads save the same file.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// `<path>` with `suffix` appended to its file name, e.g. `settings.json.bak`.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map_or_else(OsString::new, OsString::from);
    name.push(suffix);
    path.with_file_name(name)
}

/// Where the previous version of `path` is kept.
pub fn backup_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bak")
}

/// Where an unreadable `path` is moved aside, e.g. `history.json.corrupt-1700000000`.
pub fn quarantine_path(path: &Path, unix_secs: u64) -> PathBuf {
    with_suffix(path, &format!(".corrupt-{unix_secs}"))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Renames `from` over `to`, replacing it. `std::fs::rename` replaces an
/// existing target on Windows too (it uses `MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING`). A virus scanner or indexer briefly holding
/// the target can make it fail, so it is retried a few times.
fn rename_replacing(from: &Path, to: &Path) -> io::Result<()> {
    retried(|| fs::rename(from, to))
}

/// Runs `op`, trying again a few times when a file is briefly held.
fn retried<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if attempt < 4 && err.kind() != ErrorKind::NotFound => {
                attempt += 1;
                thread::sleep(Duration::from_millis(25 * attempt));
            }
            Err(err) => return Err(err),
        }
    }
}

/// Writes `bytes` to `path` without ever leaving a partly written file, and
/// keeps the version it replaces as `<name>.bak`.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = with_suffix(
        path,
        &format!(
            ".{}-{}.tmp",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ),
    );
    let result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        // Copy rather than move, so the target exists at every moment.
        match retried(|| fs::copy(path, backup_path(path))) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            // Only the backup can't be written (a read-only .bak, say). The
            // target is fine, so replacing it loses nothing; the older .bak
            // stays.
            Err(err)
                if fs::metadata(path).is_ok_and(|m| m.is_file()) && File::open(path).is_ok() =>
            {
                log!("couldn't back up {}: {err}", file_name(path));
            }
            // Refuse to replace a file we couldn't back up or even open.
            Err(err) => return Err(err),
        }
        rename_replacing(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    // Make the rename itself durable. Windows has no equivalent for folders.
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Reads and parses `path`, recovering from a damaged file.
///
/// - `Ok(Some(value))`: the file, or its backup when the file was damaged.
/// - `Ok(None)`: there is nothing to load (no file, or a damaged file and no
///   usable backup). The damaged file has been moved aside, so writing a new
///   one loses nothing.
/// - `Err(_)`: the file exists but couldn't be read or moved aside. Callers
///   must not overwrite it.
pub fn read_or_recover<T>(
    path: &Path,
    parse: impl Fn(&str) -> Result<T, String>,
) -> io::Result<Option<T>> {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let error = match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => match parse(&text) {
                Ok(value) => return Ok(Some(value)),
                Err(err) => err,
            },
            Err(_) => "not valid UTF-8".to_owned(),
        },
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            log!("couldn't read {name}: {err}");
            return Err(err);
        }
    };

    let mut quarantine = quarantine_path(path, unix_secs());
    let mut n = 1;
    while quarantine.exists() {
        quarantine = with_suffix(&quarantine_path(path, unix_secs()), &format!("-{n}"));
        n += 1;
    }
    if let Err(err) = fs::rename(path, &quarantine) {
        log!("{name} is damaged ({error}) and couldn't be moved aside: {err}");
        return Err(err);
    }
    let kept = quarantine
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    log!("{name} is damaged ({error}); kept it as {kept}");

    let backup = backup_path(path);
    let Ok(text) = fs::read_to_string(&backup) else {
        log!("no backup of {name}; starting fresh");
        return Ok(None);
    };
    match parse(&text) {
        Ok(value) => {
            // Put the backup back in place, so the next start finds it too.
            match atomic_write(path, text.as_bytes()) {
                Ok(()) => log!("restored {name} from its backup"),
                Err(err) => log!("loaded {name} from its backup but couldn't restore it: {err}"),
            }
            Ok(Some(value))
        }
        Err(err) => {
            log!("the backup of {name} is damaged too ({err}); starting fresh");
            Ok(None)
        }
    }
}

/// Removes `path`, its backup and any damaged copies moved aside, so cleared
/// data doesn't linger on disk.
pub fn remove_all(path: &Path) -> io::Result<()> {
    let remove = |p: &Path| match fs::remove_file(p) {
        Err(err) if err.kind() != ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    };
    remove(path)?;
    remove(&backup_path(path))?;
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Ok(());
    };
    let prefix = format!("{}.corrupt-", name.to_string_lossy());
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                remove(&entry.path())?;
            }
        }
    }
    Ok(())
}

/// A fresh, empty folder under the system temp folder, for tests.
#[cfg(test)]
pub fn test_dir(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "vocelle-test-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_number(text: &str) -> Result<u32, String> {
        text.trim().parse().map_err(|e| format!("{e}"))
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn an_unwritable_backup_does_not_block_saving() {
        let dir = test_dir("readonly-bak");
        let path = dir.join("data.json");
        atomic_write(&path, b"1").unwrap();
        atomic_write(&path, b"2").unwrap();
        let bak = backup_path(&path);
        let mut perms = fs::metadata(&bak).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&bak, perms.clone()).unwrap();
        atomic_write(&path, b"3").unwrap();
        assert_eq!(read_or_recover(&path, parse_number).unwrap(), Some(3));
        // The older backup is kept as it was.
        assert_eq!(fs::read_to_string(&bak).unwrap(), "1");
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = fs::set_permissions(&bak, perms);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn round_trip_keeps_previous_version_as_backup() {
        let dir = test_dir("roundtrip");
        let path = dir.join("data.json");
        atomic_write(&path, b"1").unwrap();
        assert!(!backup_path(&path).exists());
        atomic_write(&path, b"2").unwrap();
        assert_eq!(read_or_recover(&path, parse_number).unwrap(), Some(2));
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), "1");
        // No temporary files are left behind.
        assert_eq!(files_in(&dir), ["data.json", "data.json.bak"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_is_nothing_to_load() {
        let dir = test_dir("missing");
        assert_eq!(
            read_or_recover(&dir.join("none.json"), parse_number).unwrap(),
            None
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_file_is_quarantined_and_backup_restored() {
        let dir = test_dir("recover");
        let path = dir.join("data.json");
        atomic_write(&path, b"7").unwrap();
        atomic_write(&path, b"8").unwrap();
        fs::write(&path, "8garbage").unwrap();

        assert_eq!(read_or_recover(&path, parse_number).unwrap(), Some(7));
        // The backup is back in place and the damaged file is kept aside.
        assert_eq!(fs::read_to_string(&path).unwrap(), "7");
        let corrupt: Vec<String> = files_in(&dir)
            .into_iter()
            .filter(|n| n.starts_with("data.json.corrupt-"))
            .collect();
        assert_eq!(corrupt.len(), 1);
        assert_eq!(
            fs::read_to_string(dir.join(&corrupt[0])).unwrap(),
            "8garbage"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_file_without_backup_is_kept_aside() {
        let dir = test_dir("nobackup");
        let path = dir.join("data.json");
        fs::write(&path, "{").unwrap();
        assert_eq!(read_or_recover(&path, parse_number).unwrap(), None);
        assert!(!path.exists());
        assert_eq!(files_in(&dir).len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn repeated_damage_never_overwrites_a_quarantined_file() {
        let dir = test_dir("twice");
        let path = dir.join("data.json");
        fs::write(&path, "a").unwrap();
        assert_eq!(read_or_recover(&path, parse_number).unwrap(), None);
        fs::write(&path, "b").unwrap();
        assert_eq!(read_or_recover(&path, parse_number).unwrap(), None);
        assert_eq!(files_in(&dir).len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn quarantine_name_appends_the_timestamp() {
        let path = Path::new("dir").join("settings.json");
        assert_eq!(
            quarantine_path(&path, 1_700_000_000),
            Path::new("dir").join("settings.json.corrupt-1700000000")
        );
        assert_eq!(
            backup_path(&path),
            Path::new("dir").join("settings.json.bak")
        );
    }

    #[test]
    fn remove_all_clears_backup_and_damaged_copies() {
        let dir = test_dir("remove");
        let path = dir.join("history.json");
        atomic_write(&path, b"1").unwrap();
        atomic_write(&path, b"2").unwrap();
        fs::write(quarantine_path(&path, 5), "x").unwrap();
        fs::write(dir.join("stats.json"), "3").unwrap();
        remove_all(&path).unwrap();
        assert_eq!(files_in(&dir), ["stats.json"]);
        let _ = fs::remove_dir_all(dir);
    }
}
