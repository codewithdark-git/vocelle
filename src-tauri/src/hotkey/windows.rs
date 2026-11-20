//! Windows: a low-level keyboard hook.
//!
//! `RegisterHotKey` only reports key presses, so it can't tell when a key is
//! released. A `WH_KEYBOARD_LL` hook sees both, for every app, without
//! needing focus. Modifier talk keys pass through to other apps so shortcuts
//! keep working; other talk keys (Caps Lock, F-keys) are swallowed so they
//! don't also toggle caps or trigger app shortcuts.
//!
//! The key-up can be missed (e.g. the UAC prompt or an admin window takes
//! focus mid-press), so `watchdog` also checks the key's physical state, and a
//! key-down long after the last one counts as a new press, not a repeat.
//!
//! Keys Vocelle sends itself change that physical state too: the Ctrl+V of a
//! paste releases Left Ctrl as far as Windows can tell, even while the user
//! holds it as the talk key. So the state is only trusted once the key has
//! been seen down again after the last key we sent.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::Sender,
        Mutex, MutexGuard, OnceLock,
    },
    time::Instant,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL},
    },
    UI::Input::KeyboardAndMouse::GetAsyncKeyState,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    },
};

use super::{HotkeyEvent, UpWatch};
use crate::{
    keys::{self, TalkKey},
    log::log,
};

static SENDER: OnceLock<Sender<HotkeyEvent>> = OnceLock::new();
static TALK_KEY: AtomicU32 = AtomicU32::new(0);
static SWALLOW_TALK_KEY: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// Counts presses, so the watchdog never releases a newer hold than it checked.
static HOLD: AtomicU32 = AtomicU32::new(0);
/// Counts the keys Vocelle has sent itself (each changes the key state).
static INJECTIONS: AtomicU32 = AtomicU32::new(0);
/// `INJECTIONS + 1` when the watchdog has seen this hold's key physically
/// down since the last key we sent, so its physical state can be trusted (a
/// key remapper's hook can hide it, and our own Ctrl+V can fake a key-up).
/// Any other value means not trusted.
static TRACKED: AtomicU32 = AtomicU32::new(0);
/// Event time (ms) of the last talk key-down, to tell repeats from new presses.
static LAST_DOWN: AtomicU32 = AtomicU32::new(0);
/// Set by the hook when a press had to release a hold whose key-up was
/// missed; the watchdog logs it, since the hook must stay quick.
static MISSED_UP: AtomicBool = AtomicBool::new(false);
/// Keeps a release from another thread and a press from the hook in order on
/// the channel.
static ORDER: Mutex<()> = Mutex::new(());

pub fn supports(code: &str) -> bool {
    keys::windows_vk(code).is_some()
}

pub fn set_talk_key(key: &TalkKey) -> Result<(), String> {
    let vk = keys::windows_vk(key.code).ok_or_else(|| format!("{} isn't supported", key.label))?;
    // A key change mid-press would leave HELD stuck; release it first.
    let _order = order();
    untrack();
    if HELD.swap(false, Ordering::Relaxed) {
        send(HotkeyEvent::Released);
    }
    SWALLOW_TALK_KEY.store(!key.modifier, Ordering::Relaxed);
    TALK_KEY.store(vk, Ordering::Relaxed);
    Ok(())
}

/// Installs the hook on its own thread, which must keep pumping messages for the
/// hook to fire. Events are delivered on `tx`.
pub fn start(tx: Sender<HotkeyEvent>) -> std::io::Result<()> {
    if SENDER.set(tx).is_err() {
        return Err(std::io::Error::other("hotkey hook already started"));
    }
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("hotkey-hook".into())
        .spawn(move || unsafe {
            // Windows skips a low-level hook that doesn't answer within about
            // 300 ms, and the key press is then lost. While Whisper keeps every
            // core busy, a normal-priority hook thread can wait that long, so
            // a press made during a transcription went missing. The hook does
            // almost no work, so running it first costs nothing.
            if let Err(err) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) {
                log!("couldn't raise the keyboard hook's priority: {err}");
            }
            let module = GetModuleHandleW(None).unwrap_or_default();
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(module.into()), 0);
            if let Err(err) = hook {
                super::set_failed(format!("couldn't install the keyboard hook: {err}"));
                let _ = ready_tx.send(());
                return;
            }
            let _ = ready_tx.send(());
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })?;
    // Wait briefly for the hook, so a failure is known before the Settings
    // window first asks. It's still recorded if it comes later.
    let _ = ready_rx.recv_timeout(std::time::Duration::from_secs(2));
    Ok(())
}

/// Releases the talk key if it has been physically up for a while but its
/// key-up never reached the hook.
///
/// Swallowed talk keys (Caps Lock, F-keys) never reach the physical key state,
/// because the hook stops them first, so they can't be checked this way. For
/// those, a missed key-up is caught by the next press (see `hook_proc`) or by
/// the recording limit.
pub fn watchdog() {
    static WATCH: Mutex<UpWatch> = Mutex::new(UpWatch::new());
    if MISSED_UP.swap(false, Ordering::Relaxed) {
        log!("talk key-up was missed; the next press released it");
    }
    if !HELD.load(Ordering::Relaxed) || SWALLOW_TALK_KEY.load(Ordering::Relaxed) {
        return;
    }
    let hold = HOLD.load(Ordering::Relaxed);
    // Read before the key state: a key we send after this makes the state
    // read now count for an older epoch, so it isn't trusted.
    let epoch = INJECTIONS.load(Ordering::Relaxed);
    let down = physically_down(TALK_KEY.load(Ordering::Relaxed));
    if down {
        TRACKED.store(epoch.wrapping_add(1), Ordering::Relaxed);
    }
    let missed = WATCH.lock().unwrap_or_else(|e| e.into_inner()).missed_up(
        hold,
        epoch,
        down,
        Instant::now(),
    );
    if missed {
        let released = {
            let _order = order();
            let release = HOLD.load(Ordering::Relaxed) == hold
                && INJECTIONS.load(Ordering::Relaxed) == epoch
                && HELD.swap(false, Ordering::Relaxed);
            if release {
                send(HotkeyEvent::Released);
            }
            release
        };
        // Logged after letting go of ORDER: the hook waits on it, and a slow
        // log write mustn't hold up the hook.
        if released {
            log!("talk key-up was missed; released the key");
        }
    }
}

/// Whether the watchdog has seen the key down since the last key we sent.
fn tracked() -> bool {
    TRACKED.load(Ordering::Relaxed) == INJECTIONS.load(Ordering::Relaxed).wrapping_add(1)
}

/// Stops trusting the key's physical state until the watchdog sees it down.
fn untrack() {
    TRACKED.store(INJECTIONS.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Whether the key is down right now. Inside the hook this is the state
/// before the event being handled.
fn physically_down(vk: u32) -> bool {
    // The high bit (the sign) is set while the key is down.
    unsafe { GetAsyncKeyState(vk as i32) < 0 }
}

fn order() -> MutexGuard<'static, ()> {
    ORDER.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set `VOCELLE_TEST_INJECTED_KEYS=1` in a debug build to drive the talk key with `SendInput`.
fn accept_injected_talk_key() -> bool {
    static ACCEPT: OnceLock<bool> = OnceLock::new();
    *ACCEPT.get_or_init(|| {
        cfg!(debug_assertions) && std::env::var_os("VOCELLE_TEST_INJECTED_KEYS").is_some()
    })
}

fn send(event: HotkeyEvent) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(event);
    }
}

// Runs on the hook thread for every key event system-wide. Windows drops the
// hook if it takes too long, so it only flips flags and sends on a channel.
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_talk_key = info.vkCode == TALK_KEY.load(Ordering::Relaxed);
        // Ignore keys we inject ourselves (the Ctrl+V used for pasting). Debug
        // builds can let an injected talk key through for automated testing.
        let injected = info.flags.0 & LLKHF_INJECTED.0 != 0;
        if injected {
            // The key state no longer shows what the user's fingers do.
            INJECTIONS.fetch_add(1, Ordering::Relaxed);
        }
        if !injected || (is_talk_key && accept_injected_talk_key()) {
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

            if is_talk_key && down {
                let _order = order();
                let last = LAST_DOWN.swap(info.time, Ordering::Relaxed);
                // A key-down while we think the key is held is normally a
                // repeat. It's a new press if the key-up was missed: repeats
                // stopped long ago, or the key was seen up just before this.
                if HELD.load(Ordering::Relaxed)
                    && (super::is_new_press(last, info.time)
                        || (tracked() && !physically_down(info.vkCode)))
                {
                    HELD.store(false, Ordering::Relaxed);
                    MISSED_UP.store(true, Ordering::Relaxed);
                    send(HotkeyEvent::Released);
                }
                // Holding a key auto-repeats key-down; report only the first.
                if !HELD.swap(true, Ordering::Relaxed) {
                    INTERRUPTED.store(false, Ordering::Relaxed);
                    untrack();
                    HOLD.fetch_add(1, Ordering::Relaxed);
                    send(HotkeyEvent::Pressed);
                }
            } else if is_talk_key && up {
                let _order = order();
                if HELD.swap(false, Ordering::Relaxed) {
                    send(HotkeyEvent::Released);
                }
            } else if down
                && HELD.load(Ordering::Relaxed)
                && !INTERRUPTED.swap(true, Ordering::Relaxed)
            {
                send(HotkeyEvent::Interrupted);
            }

            if is_talk_key && (down || up) && SWALLOW_TALK_KEY.load(Ordering::Relaxed) {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
