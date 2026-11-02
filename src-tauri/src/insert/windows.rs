//! Windows: Ctrl+V or Unicode keystrokes through `SendInput`, plus a check for
//! elevated target windows, which Windows protects from our input.

use std::mem::size_of;

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::{
        DataExchange::{CloseClipboard, EnumClipboardFormats, OpenClipboard},
        Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL, VK_RETURN, VK_V,
        },
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

use super::InsertError;

/// Plain-text clipboard formats: CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT, CF_LOCALE.
const TEXT_FORMATS: [u32; 4] = [1, 7, 13, 16];
/// `SendInput` batch size; very long batches can be dropped by some apps.
const TYPE_BATCH: usize = 256;

pub fn check_target() -> Result<(), InsertError> {
    if foreground_is_elevated() && !process_is_elevated(unsafe { GetCurrentProcess() }) {
        return Err(InsertError::ElevatedTarget);
    }
    Ok(())
}

/// True when the clipboard is empty or holds only plain text, so pasting and
/// restoring the text loses nothing.
pub fn clipboard_is_text_only(_clipboard: &mut arboard::Clipboard) -> bool {
    unsafe {
        if OpenClipboard(None).is_err() {
            // Another app is holding the clipboard; typing is the safe choice.
            return false;
        }
        let mut format = 0;
        let mut text_only = true;
        loop {
            format = EnumClipboardFormats(format);
            if format == 0 {
                break;
            }
            if !TEXT_FORMATS.contains(&format) {
                text_only = false;
                break;
            }
        }
        let _ = CloseClipboard();
        text_only
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    keyboard_input(
        vk,
        0,
        if up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        },
    )
}

fn keyboard_input(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), InsertError> {
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(InsertError::Keystrokes(
            "Windows blocked the keystrokes".into(),
        ))
    }
}

pub fn send_paste() -> Result<(), InsertError> {
    send(&[
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ])
}

/// Types `text` as Unicode keystrokes. Line breaks are sent as Enter.
pub fn type_text(text: &str) -> Result<(), InsertError> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for c in text.chars() {
        if c == '\n' {
            inputs.push(key(VK_RETURN, false));
            inputs.push(key(VK_RETURN, true));
            continue;
        }
        if c == '\r' {
            continue;
        }
        let mut units = [0u16; 2];
        for unit in c.encode_utf16(&mut units) {
            inputs.push(keyboard_input(VIRTUAL_KEY(0), *unit, KEYEVENTF_UNICODE));
            inputs.push(keyboard_input(
                VIRTUAL_KEY(0),
                *unit,
                KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            ));
        }
    }
    for batch in inputs.chunks(TYPE_BATCH) {
        send(batch)?;
    }
    Ok(())
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
