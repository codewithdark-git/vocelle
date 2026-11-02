//! Puts transcribed text into whatever app has focus.
//!
//! Pasting is fast and works everywhere, so when the clipboard holds only text,
//! the dictation is pasted and the previous text is put back. When it holds
//! anything else (an image, files, formatted text), the dictation is typed
//! instead, so the clipboard is never touched.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use windows as platform;

use std::{thread, time::Duration};

/// How long to wait before restoring the clipboard. The target app reads the
/// clipboard asynchronously after Ctrl+V; restoring too early pastes the old text.
const RESTORE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum InsertError {
    /// The focused window runs as administrator and we don't, so Windows
    /// blocks our keystrokes (User Interface Privilege Isolation).
    ElevatedTarget,
    Clipboard(String),
    Keystrokes(String),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElevatedTarget => write!(f, "Can't type into an app running as administrator"),
            Self::Clipboard(e) => write!(f, "Clipboard error: {e}"),
            Self::Keystrokes(e) => write!(f, "Couldn't send keystrokes: {e}"),
        }
    }
}

pub fn insert_text(text: &str) -> Result<(), InsertError> {
    if text.is_empty() {
        return Ok(());
    }
    platform::check_target()?;

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InsertError::Clipboard(e.to_string()))?;
    if !platform::clipboard_is_text_only(&mut clipboard) {
        return platform::type_text(text);
    }

    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;
    let sent = platform::send_paste();

    thread::sleep(RESTORE_DELAY);
    match previous {
        Some(prev) => {
            let _ = clipboard.set_text(prev);
        }
        None => {
            let _ = clipboard.clear();
        }
    }
    sent
}
