//! Puts transcribed text into whatever app has focus.
//!
//! Pasting is fast and works everywhere, so the dictation goes on the
//! clipboard, Ctrl+V is sent, and the user's clipboard is put back afterwards.
//! The platform modules decide how much of the clipboard they can preserve and
//! fall back to typing the text when they can't.
//!
//! The clipboard is put back on a separate thread, so the next dictation
//! doesn't wait for it. It's put back only while the clipboard still holds
//! the dictation: anything the user copied meanwhile is kept. When the next
//! dictation is pasted before that, the user's clipboard stays saved and is
//! put back after that one instead. Nothing replaces the clipboard until the
//! last paste has had `RESTORE_DELAY` to read it, or that app could paste the
//! newer text instead.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use windows as platform;

use std::{
    sync::{Condvar, Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};

use crate::log::log;

/// How long to wait before restoring the clipboard. The target app reads the
/// clipboard asynchronously after Ctrl+V; restoring too early pastes the old content.
const RESTORE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum InsertError {
    /// The focused window runs as administrator and we don't, so Windows
    /// blocks our keystrokes (User Interface Privilege Isolation).
    #[cfg(windows)]
    ElevatedTarget,
    Clipboard(String),
    Keystrokes(String),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(windows)]
            Self::ElevatedTarget => write!(f, "Can't type into an app running as administrator"),
            Self::Clipboard(e) => write!(f, "Clipboard error: {e}"),
            Self::Keystrokes(e) => write!(f, "Couldn't send keystrokes: {e}"),
        }
    }
}

/// The user's clipboard, saved before a paste and waiting to be put back.
struct Pending {
    saved: platform::Saved,
    due: Instant,
}

/// Held while the clipboard is in use, so a dictation, "Paste last
/// dictation" and the restore can't swap the clipboard under each other.
/// It also holds the restore that's waiting.
static CLIPBOARD: Mutex<Option<Pending>> = Mutex::new(None);
/// Wakes the restore thread when a restore is scheduled.
static RESTORE_DUE: Condvar = Condvar::new();
static RESTORER: OnceLock<bool> = OnceLock::new();

fn clipboard_lock() -> MutexGuard<'static, Option<Pending>> {
    CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn insert_text(text: &str) -> Result<(), InsertError> {
    if text.is_empty() {
        return Ok(());
    }
    platform::check_target()?;
    let mut clipboard = clipboard_lock();
    wait_for_last_paste(&clipboard);
    // A restore still waiting means the clipboard may hold the previous
    // dictation; the platform keeps the user's clipboard saved, not that.
    let mut saved = clipboard.take().map(|p| p.saved);
    let result = platform::insert(text, &mut saved);
    if let Some(saved) = saved {
        if start_restorer() {
            *clipboard = Some(Pending {
                saved,
                due: Instant::now() + RESTORE_DELAY,
            });
            RESTORE_DUE.notify_one();
        } else {
            // No restore thread: put the clipboard back here, as before.
            std::thread::sleep(RESTORE_DELAY);
            platform::restore(saved);
        }
    }
    result
}

/// Puts `text` on the clipboard and leaves it there for the user to paste,
/// for when it couldn't be typed.
pub fn copy_text(text: &str) -> Result<(), InsertError> {
    let mut clipboard = clipboard_lock();
    wait_for_last_paste(&clipboard);
    // The text is meant to stay, so a waiting restore mustn't replace it.
    *clipboard = None;
    platform::copy(text)
}

/// Copies text the user asked for (the Copy button on Home). Unlike a
/// dictation, it's allowed into clipboard history.
pub fn copy_for_user(text: &str) -> Result<(), InsertError> {
    let mut clipboard = clipboard_lock();
    wait_for_last_paste(&clipboard);
    // A waiting restore mustn't replace what the user just copied.
    *clipboard = None;
    platform::copy_plain(text)
}

/// Waits until the app that got the last Ctrl+V has had time to read the
/// clipboard, so changing it now can't change what's pasted there. Called
/// with the lock held, so the restore waits too.
fn wait_for_last_paste(pending: &Option<Pending>) {
    if let Some(due) = pending.as_ref().map(|p| p.due) {
        let left = due.saturating_duration_since(Instant::now());
        if !left.is_zero() {
            std::thread::sleep(left);
        }
    }
}

/// Starts the restore thread the first time; returns whether it's running.
fn start_restorer() -> bool {
    *RESTORER.get_or_init(|| {
        let spawned = std::thread::Builder::new()
            .name("clipboard-restore".into())
            .spawn(restore_when_due);
        if let Err(err) = &spawned {
            log!("couldn't start the clipboard restore thread: {err}");
        }
        spawned.is_ok()
    })
}

/// Puts the user's clipboard back once each paste has had time to read the
/// dictation. Runs for the life of the app.
fn restore_when_due() {
    let mut clipboard = clipboard_lock();
    loop {
        clipboard = match clipboard.as_ref().map(|p| p.due) {
            None => RESTORE_DUE
                .wait(clipboard)
                .unwrap_or_else(|e| e.into_inner()),
            Some(due) if Instant::now() < due => {
                RESTORE_DUE
                    .wait_timeout(clipboard, due.saturating_duration_since(Instant::now()))
                    .unwrap_or_else(|e| e.into_inner())
                    .0
            }
            Some(_) => {
                // Restored with the lock held, so no paste starts meanwhile.
                if let Some(pending) = clipboard.take() {
                    platform::restore(pending.saved);
                }
                clipboard
            }
        };
    }
}

/// Whether another app's window has keyboard focus, so a paste would land
/// there and not in Vocelle's own window, the taskbar or the tray menu.
pub fn focus_is_elsewhere() -> bool {
    platform::focus_is_elsewhere()
}
