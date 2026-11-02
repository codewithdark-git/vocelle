//! Keys that can be used as the talk key.
//!
//! Keys are named with the web `KeyboardEvent.code` values ("ControlRight",
//! "F9"), so the Settings page can capture a key press and send its code
//! straight to the backend. Letters, digits and Space are excluded on purpose:
//! holding them would type characters into the target app.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct TalkKey {
    pub code: &'static str,
    pub label: &'static str,
    /// Modifiers must reach other apps so shortcuts like Ctrl+C keep working.
    /// Other keys are swallowed while they act as the talk key (Windows only).
    pub modifier: bool,
}

const fn key(code: &'static str, label: &'static str, modifier: bool) -> TalkKey {
    TalkKey {
        code,
        label,
        modifier,
    }
}

pub const DEFAULT: &str = "ControlRight";

pub const TALK_KEYS: &[TalkKey] = &[
    key("ControlRight", "Right Ctrl", true),
    key("ControlLeft", "Left Ctrl", true),
    key("ShiftRight", "Right Shift", true),
    key("CapsLock", "Caps Lock", false),
    key("ScrollLock", "Scroll Lock", false),
    key("Pause", "Pause", false),
    key("Insert", "Insert", false),
    key("ContextMenu", "Menu", false),
    key("F1", "F1", false),
    key("F2", "F2", false),
    key("F3", "F3", false),
    key("F4", "F4", false),
    key("F5", "F5", false),
    key("F6", "F6", false),
    key("F7", "F7", false),
    key("F8", "F8", false),
    key("F9", "F9", false),
    key("F10", "F10", false),
    key("F11", "F11", false),
    key("F12", "F12", false),
    key("F13", "F13", false),
    key("F14", "F14", false),
    key("F15", "F15", false),
    key("F16", "F16", false),
    key("F17", "F17", false),
    key("F18", "F18", false),
    key("F19", "F19", false),
    key("F20", "F20", false),
    key("F21", "F21", false),
    key("F22", "F22", false),
    key("F23", "F23", false),
    key("F24", "F24", false),
];

pub fn find(code: &str) -> Option<&'static TalkKey> {
    TALK_KEYS.iter().find(|k| k.code == code)
}

/// Windows virtual-key code for a talk key.
#[cfg(windows)]
pub fn windows_vk(code: &str) -> Option<u32> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let vk = match code {
        "ControlRight" => VK_RCONTROL,
        "ControlLeft" => VK_LCONTROL,
        "ShiftRight" => VK_RSHIFT,
        "CapsLock" => VK_CAPITAL,
        "ScrollLock" => VK_SCROLL,
        "Pause" => VK_PAUSE,
        "Insert" => VK_INSERT,
        "ContextMenu" => VK_APPS,
        f if f.starts_with('F') => {
            let n: u16 = f[1..].parse().ok().filter(|n| (1..=24).contains(n))?;
            VIRTUAL_KEY(VK_F1.0 + n - 1)
        }
        _ => return None,
    };
    Some(vk.0 as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_listed() {
        assert!(find(DEFAULT).is_some());
    }

    #[test]
    fn codes_are_unique() {
        for (i, k) in TALK_KEYS.iter().enumerate() {
            assert!(TALK_KEYS[i + 1..].iter().all(|o| o.code != k.code));
        }
    }

    #[cfg(windows)]
    #[test]
    fn every_key_maps_to_a_windows_vk() {
        for k in TALK_KEYS {
            assert!(windows_vk(k.code).is_some(), "{}", k.code);
        }
        assert_eq!(windows_vk("F1"), Some(0x70));
        assert_eq!(windows_vk("F24"), Some(0x87));
        assert_eq!(windows_vk("F25"), None);
        assert_eq!(windows_vk("KeyA"), None);
    }
}
