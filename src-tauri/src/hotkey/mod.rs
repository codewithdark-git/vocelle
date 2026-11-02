//! Global talk key: reports when it goes down and up, from any app, without focus.
//!
//! The platform modules do the listening; this module is the shared interface.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use windows as platform;

#[cfg(target_os = "linux")]
pub use linux::while_injecting;

use std::sync::mpsc::Sender;

use crate::keys::{self, TalkKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// The talk key went down.
    Pressed,
    /// The talk key came back up.
    Released,
    /// Another key was pressed while the talk key was held (e.g. Right Ctrl+C),
    /// so the user meant a shortcut, not dictation.
    Interrupted,
}

/// Starts listening on a background thread. Events are delivered on `tx`.
pub fn start(tx: Sender<HotkeyEvent>, talk_key: &str) -> std::io::Result<()> {
    if let Err(err) = set_talk_key(talk_key) {
        eprintln!("vocelle: {err}; using {}", keys::DEFAULT);
        let _ = set_talk_key(keys::DEFAULT);
    }
    platform::start(tx)
}

/// Changes the talk key immediately; no restart needed.
pub fn set_talk_key(code: &str) -> Result<(), String> {
    let key = keys::find(code).ok_or_else(|| format!("{code} can't be used as the talk key"))?;
    platform::set_talk_key(key)
}

/// Talk keys this platform can listen for.
pub fn available_keys() -> Vec<TalkKey> {
    keys::TALK_KEYS
        .iter()
        .filter(|k| platform::supports(k.code))
        .copied()
        .collect()
}
