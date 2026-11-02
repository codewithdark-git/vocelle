//! Puts transcribed text into whatever app has focus.
//!
//! Pasting is fast and works everywhere, so the dictation goes on the
//! clipboard, Ctrl+V is sent, and the user's clipboard is put back afterwards.
//! The platform modules decide how much of the clipboard they can preserve and
//! fall back to typing the text when they can't.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use windows as platform;

use std::time::Duration;

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

pub fn insert_text(text: &str) -> Result<(), InsertError> {
    if text.is_empty() {
        return Ok(());
    }
    platform::check_target()?;
    platform::insert(text)
}
