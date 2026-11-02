//! Linux: global key events from the X server (via rdev's XRecord listener).
//!
//! Works on X11 sessions. Wayland doesn't let apps see global key events, so
//! on Wayland the talk key only works while an X11 (XWayland) app has focus.
//! Keys can't be swallowed here, so Caps Lock as the talk key also toggles caps.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::Sender,
    OnceLock,
};

use rdev::{EventType, Key};

use super::HotkeyEvent;
use crate::keys::{TalkKey, TALK_KEYS};

static SENDER: OnceLock<Sender<HotkeyEvent>> = OnceLock::new();
/// Index into `TALK_KEYS`.
static TALK_KEY: AtomicUsize = AtomicUsize::new(0);
static HELD: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// Set while Vocelle itself sends keys, which X11 reports like real ones.
static INJECTING: AtomicBool = AtomicBool::new(false);

/// Runs `f` (which sends simulated keys) without those keys reaching the listener.
pub fn while_injecting<R>(f: impl FnOnce() -> R) -> R {
    INJECTING.store(true, Ordering::SeqCst);
    let result = f();
    // Let the X server deliver the simulated events before listening again.
    std::thread::sleep(std::time::Duration::from_millis(50));
    INJECTING.store(false, Ordering::SeqCst);
    result
}

fn rdev_key(code: &str) -> Option<Key> {
    Some(match code {
        "ControlRight" => Key::ControlRight,
        "ControlLeft" => Key::ControlLeft,
        "ShiftRight" => Key::ShiftRight,
        "CapsLock" => Key::CapsLock,
        "ScrollLock" => Key::ScrollLock,
        "Pause" => Key::Pause,
        "Insert" => Key::Insert,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        _ => return None,
    })
}

pub fn supports(code: &str) -> bool {
    rdev_key(code).is_some()
}

pub fn set_talk_key(key: &TalkKey) -> Result<(), String> {
    let index = TALK_KEYS
        .iter()
        .position(|k| k.code == key.code && rdev_key(k.code).is_some())
        .ok_or_else(|| format!("{} isn't supported on Linux", key.label))?;
    if HELD.swap(false, Ordering::Relaxed) {
        send(HotkeyEvent::Released);
    }
    TALK_KEY.store(index, Ordering::Relaxed);
    Ok(())
}

pub fn start(tx: Sender<HotkeyEvent>) -> std::io::Result<()> {
    if SENDER.set(tx).is_err() {
        return Err(std::io::Error::other("hotkey listener already started"));
    }
    std::thread::Builder::new()
        .name("hotkey-listen".into())
        .spawn(|| {
            if let Err(err) = rdev::listen(on_event) {
                eprintln!("vocelle: can't listen for the talk key: {err:?}");
            }
        })?;
    Ok(())
}

fn send(event: HotkeyEvent) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(event);
    }
}

fn on_event(event: rdev::Event) {
    if INJECTING.load(Ordering::SeqCst) {
        return;
    }
    let talk_key = rdev_key(TALK_KEYS[TALK_KEY.load(Ordering::Relaxed)].code);
    match event.event_type {
        EventType::KeyPress(key) if Some(key) == talk_key => {
            // Auto-repeat sends repeated presses; report only the first.
            if !HELD.swap(true, Ordering::Relaxed) {
                INTERRUPTED.store(false, Ordering::Relaxed);
                send(HotkeyEvent::Pressed);
            }
        }
        EventType::KeyRelease(key) if Some(key) == talk_key => {
            if HELD.swap(false, Ordering::Relaxed) {
                send(HotkeyEvent::Released);
            }
        }
        EventType::KeyPress(_)
            if HELD.load(Ordering::Relaxed) && !INTERRUPTED.swap(true, Ordering::Relaxed) =>
        {
            send(HotkeyEvent::Interrupted);
        }
        _ => {}
    }
}
