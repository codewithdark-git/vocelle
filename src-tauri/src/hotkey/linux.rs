//! Linux: global key events from the X server (via rdev's XRecord listener).
//!
//! Works on X11 sessions. Wayland doesn't let apps see global key events, so
//! on Wayland the talk key only works while an X11 (XWayland) app has focus.
//! Keys can't be swallowed here, so Caps Lock as the talk key also toggles caps.
//! A missed key-up is caught by the next press: a key-down long after the
//! last one counts as a new press, not a repeat.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        mpsc::Sender,
        OnceLock,
    },
    time::Instant,
};

use rdev::{EventType, Key};

use super::HotkeyEvent;
use crate::{
    keys::{TalkKey, TALK_KEYS},
    log::log,
};

static SENDER: OnceLock<Sender<HotkeyEvent>> = OnceLock::new();
/// Index into `TALK_KEYS`.
static TALK_KEY: AtomicUsize = AtomicUsize::new(0);
static HELD: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// When (ms since `START`) the last talk key-down came, to tell repeats from
/// new presses.
static LAST_DOWN: AtomicU32 = AtomicU32::new(0);
static START: OnceLock<Instant> = OnceLock::new();
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
    log!("no talk key watchdog on Linux yet; a missed key-up is caught on the next press");
    std::thread::Builder::new()
        .name("hotkey-listen".into())
        .spawn(|| {
            if let Err(err) = rdev::listen(on_event) {
                super::set_failed(format!("can't listen for key events: {err:?}"));
            }
        })?;
    Ok(())
}

/// TODO: check the key's physical state (XQueryKeymap) like Windows does, so
/// a missed key-up is caught without waiting for the next press. It needs
/// the x11 crate as a direct dependency.
pub fn watchdog() {}

fn now_ms() -> u32 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
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
            let now = now_ms();
            let last = LAST_DOWN.swap(now, Ordering::Relaxed);
            // Repeats stopped long ago, so the key-up was missed (e.g. it
            // came while Vocelle was typing): this is a new press.
            if HELD.load(Ordering::Relaxed) && super::is_new_press(last, now) {
                log!("talk key-up was missed; the next press released it");
                HELD.store(false, Ordering::Relaxed);
                send(HotkeyEvent::Released);
            }
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
