//! Windows: a low-level keyboard hook.
//!
//! `RegisterHotKey` only reports key presses, so it can't tell when a key is
//! released. A `WH_KEYBOARD_LL` hook sees both, for every app, without
//! needing focus. Modifier talk keys pass through to other apps so shortcuts
//! keep working; other talk keys (Caps Lock, F-keys) are swallowed so they
//! don't also toggle caps or trigger app shortcuts.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc::Sender,
    OnceLock,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    },
};

use super::HotkeyEvent;
use crate::keys::{self, TalkKey};

static SENDER: OnceLock<Sender<HotkeyEvent>> = OnceLock::new();
static TALK_KEY: AtomicU32 = AtomicU32::new(0);
static SWALLOW_TALK_KEY: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub fn supports(code: &str) -> bool {
    keys::windows_vk(code).is_some()
}

pub fn set_talk_key(key: &TalkKey) -> Result<(), String> {
    let vk = keys::windows_vk(key.code).ok_or_else(|| format!("{} isn't supported", key.label))?;
    // A key change mid-press would leave HELD stuck; release it first.
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
    std::thread::Builder::new()
        .name("hotkey-hook".into())
        .spawn(|| unsafe {
            let module = GetModuleHandleW(None).unwrap_or_default();
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(module.into()), 0);
            if let Err(err) = hook {
                eprintln!("vocelle: failed to install keyboard hook: {err}");
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })?;
    Ok(())
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
        if !injected || (is_talk_key && accept_injected_talk_key()) {
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

            if is_talk_key && down {
                // Holding a key auto-repeats key-down; report only the first.
                if !HELD.swap(true, Ordering::Relaxed) {
                    INTERRUPTED.store(false, Ordering::Relaxed);
                    send(HotkeyEvent::Pressed);
                }
            } else if is_talk_key && up {
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
