//! Puts transcribed text into whatever app has focus.
//!
//! Pasting is faster and more reliable than typing characters one by one, so
//! the text goes on the clipboard, Ctrl+V is sent, and the user's previous
//! clipboard text is put back afterwards.

use std::{mem::size_of, thread, time::Duration};

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_V,
        },
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

/// How long to wait before restoring the clipboard. The target app reads the
/// clipboard asynchronously after Ctrl+V; restoring too early pastes the old text.
const RESTORE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum InsertError {
    /// The focused window runs as administrator and we don't, so Windows
    /// blocks our keystrokes (User Interface Privilege Isolation).
    ElevatedTarget,
    Clipboard(String),
    SendInput,
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElevatedTarget => write!(f, "Can't type into an app running as administrator"),
            Self::Clipboard(e) => write!(f, "Clipboard error: {e}"),
            Self::SendInput => write!(f, "Windows blocked the paste keystroke"),
        }
    }
}

pub fn insert_text(text: &str) -> Result<(), InsertError> {
    if text.is_empty() {
        return Ok(());
    }
    if foreground_is_elevated() && !process_is_elevated(unsafe { GetCurrentProcess() }) {
        return Err(InsertError::ElevatedTarget);
    }

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InsertError::Clipboard(e.to_string()))?;
    // Only text is preserved for now; images or files on the clipboard are lost.
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;

    let sent = send_ctrl_v();

    thread::sleep(RESTORE_DELAY);
    match previous {
        Some(prev) => {
            let _ = clipboard.set_text(prev);
        }
        None => {
            let _ = clipboard.clear();
        }
    }

    if sent {
        Ok(())
    } else {
        Err(InsertError::SendInput)
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                ..Default::default()
            },
        },
    }
}

fn send_ctrl_v() -> bool {
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    sent as usize == inputs.len()
}

fn foreground_is_elevated() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return false;
        }
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let elevated = process_is_elevated(process);
        let _ = CloseHandle(process);
        elevated
    }
}

fn process_is_elevated(process: HANDLE) -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}
