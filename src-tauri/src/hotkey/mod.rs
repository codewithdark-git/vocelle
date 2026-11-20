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

use std::sync::{mpsc::Sender, Mutex};
#[cfg(any(windows, test))]
use std::time::{Duration, Instant};

use crate::{
    keys::{self, TalkKey},
    log::log,
};

/// A talk key that has been physically up this long while we think it's held
/// lost its key-up (e.g. focus moved to an admin window or the UAC prompt
/// mid-press), so it's released as if the key-up had arrived.
#[cfg(any(windows, test))]
const MISSED_UP_AFTER: Duration = Duration::from_millis(150);
/// Auto-repeat key-downs come at most this far apart (Windows waits up to
/// 1 s before repeating, 2 s with Filter Keys; X11 defaults to 660 ms). A
/// key-down for a key we think is held that comes later than this is a new
/// press after a missed key-up, not a repeat.
const REPEAT_GAP_MAX_MS: u32 = 2500;

/// Why the talk key listener couldn't start, if it couldn't.
static FAILURE: Mutex<Option<String>> = Mutex::new(None);

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
        log!("{err}; using {}", keys::DEFAULT);
        let _ = set_talk_key(keys::DEFAULT);
    }
    platform::start(tx)
}

/// Records that the listener failed, so the Settings window can say so.
/// Called by the platform modules, possibly from their own thread.
fn set_failed(reason: String) {
    log!("talk key unavailable: {reason}");
    *FAILURE.lock().unwrap_or_else(|e| e.into_inner()) = Some(reason);
}

/// Why the talk key isn't working, or `None` while it is.
pub fn failure() -> Option<String> {
    FAILURE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Releases the talk key if it was let go but the key-up never arrived.
/// Called regularly (every few tens of ms) by the dictation thread.
pub fn watchdog() {
    platform::watchdog()
}

/// Whether a key-down for a key we think is held is a new press whose key-up
/// was missed, given when the previous key-down came (in ms, wrapping).
fn is_new_press(last_down_ms: u32, now_ms: u32) -> bool {
    now_ms.wrapping_sub(last_down_ms) > REPEAT_GAP_MAX_MS
}

/// Decides, from regular looks at the talk key's physical state, when a hold
/// has lost its key-up.
#[cfg(any(windows, test))]
#[derive(Debug)]
struct UpWatch {
    /// Which hold this is about; a new hold starts over.
    hold: u32,
    /// How many keys Vocelle had sent itself. Sending one (a paste's Ctrl+V)
    /// can make a held key read as up, so a new count starts over too.
    epoch: u32,
    /// The key was seen physically down during this hold.
    seen_down: bool,
    /// When the key was first seen up since it was last seen down.
    up_since: Option<Instant>,
}

#[cfg(any(windows, test))]
impl UpWatch {
    const fn new() -> Self {
        UpWatch {
            hold: 0,
            epoch: 0,
            seen_down: false,
            up_since: None,
        }
    }

    /// Takes one look at the key during hold number `hold`, after `epoch`
    /// keys sent by Vocelle. Returns true once it has been up for
    /// `MISSED_UP_AFTER`, so the hold should be released.
    fn missed_up(&mut self, hold: u32, epoch: u32, down: bool, now: Instant) -> bool {
        if hold != self.hold || epoch != self.epoch {
            *self = UpWatch {
                hold,
                epoch,
                ..UpWatch::new()
            };
        }
        if down {
            self.seen_down = true;
            self.up_since = None;
            return false;
        }
        // "Up" is only trusted once the key was seen down this hold: a key
        // remapper's own hook can hide the key from the physical state.
        if !self.seen_down {
            return false;
        }
        let since = *self.up_since.get_or_insert(now);
        now.duration_since(since) >= MISSED_UP_AFTER
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn missed_up_needs_the_key_up_long_enough() {
        let t = Instant::now();
        let mut w = UpWatch::new();
        assert!(!w.missed_up(1, 0, true, t));
        assert!(!w.missed_up(1, 0, false, t + ms(50)));
        assert!(!w.missed_up(1, 0, false, t + ms(150)));
        assert!(w.missed_up(1, 0, false, t + ms(200)));
    }

    #[test]
    fn missed_up_restarts_when_the_key_is_down_again() {
        let t = Instant::now();
        let mut w = UpWatch::new();
        assert!(!w.missed_up(1, 0, true, t));
        assert!(!w.missed_up(1, 0, false, t + ms(50)));
        assert!(!w.missed_up(1, 0, true, t + ms(100)));
        assert!(!w.missed_up(1, 0, false, t + ms(150)));
        assert!(!w.missed_up(1, 0, false, t + ms(250)));
        assert!(w.missed_up(1, 0, false, t + ms(300)));
    }

    #[test]
    fn missed_up_ignores_a_key_never_seen_down() {
        // A remapper hiding the key: the physical state never shows it down.
        let t = Instant::now();
        let mut w = UpWatch::new();
        for n in 0..100 {
            assert!(!w.missed_up(1, 0, false, t + ms(n * 50)));
        }
    }

    #[test]
    fn missed_up_starts_over_for_a_new_hold() {
        let t = Instant::now();
        let mut w = UpWatch::new();
        assert!(!w.missed_up(1, 0, true, t));
        assert!(!w.missed_up(1, 0, false, t + ms(50)));
        // A new hold the watchdog hasn't seen down yet.
        assert!(!w.missed_up(2, 0, false, t + ms(300)));
        assert!(!w.missed_up(2, 0, false, t + ms(600)));
        assert!(!w.missed_up(2, 0, true, t + ms(650)));
        assert!(!w.missed_up(2, 0, false, t + ms(700)));
        assert!(w.missed_up(2, 0, false, t + ms(850)));
    }

    #[test]
    fn missed_up_starts_over_after_a_key_we_sent() {
        // Talk key Left Ctrl: our paste's Ctrl+V makes it read as up while
        // the user still holds it. That "up" isn't trusted.
        let t = Instant::now();
        let mut w = UpWatch::new();
        assert!(!w.missed_up(1, 0, true, t));
        for n in 1..20 {
            assert!(!w.missed_up(1, 4, false, t + ms(n * 50)));
        }
        // Seen down again (the key's auto-repeat), then really let go.
        assert!(!w.missed_up(1, 4, true, t + ms(1000)));
        assert!(!w.missed_up(1, 4, false, t + ms(1050)));
        assert!(w.missed_up(1, 4, false, t + ms(1200)));
    }

    #[test]
    fn auto_repeat_is_not_a_new_press() {
        assert!(!is_new_press(1000, 1033));
        // The longest repeat delay (Filter Keys).
        assert!(!is_new_press(1000, 3000));
        assert!(!is_new_press(1000, 3500));
        assert!(is_new_press(1000, 3501));
        assert!(is_new_press(1000, 60_000));
    }

    #[test]
    fn new_press_survives_the_tick_count_wrapping() {
        assert!(!is_new_press(u32::MAX - 10, 20));
        assert!(is_new_press(u32::MAX - 10, 3000));
    }
}
