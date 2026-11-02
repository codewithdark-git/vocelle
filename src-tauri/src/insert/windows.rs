//! Windows: paste with a full clipboard snapshot, plus a check for elevated
//! target windows, which Windows protects from our input.
//!
//! Every clipboard format (text, formatting, images, files) is copied before
//! pasting and put back afterwards, so whatever the user had copied survives.
//! The dictation itself is kept out of clipboard history and cloud sync.

use std::{mem::size_of, ptr, thread, time::Duration};

use arboard::SetExtWindows;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HGLOBAL},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, OpenClipboard,
            SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
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

use super::{InsertError, RESTORE_DELAY};

/// Clipboard formats whose data is a GDI or private handle rather than memory,
/// so it can't be copied byte for byte. Windows re-creates the common ones
/// (e.g. CF_BITMAP from CF_DIB) on its own.
fn is_handle_format(format: u32) -> bool {
    matches!(format, 2 | 3 | 9 | 14 | 0x80 | 0x82 | 0x83 | 0x8E)
        || (0x200..=0x3FF).contains(&format)
}

/// Above this, the clipboard isn't copied (a huge image, say); the dictation is typed instead.
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
/// `SendInput` batch size; very long batches can be dropped by some apps.
const TYPE_BATCH: usize = 256;

pub fn check_target() -> Result<(), InsertError> {
    if foreground_is_elevated() && !process_is_elevated(unsafe { GetCurrentProcess() }) {
        return Err(InsertError::ElevatedTarget);
    }
    Ok(())
}

pub fn insert(text: &str) -> Result<(), InsertError> {
    let Some(snapshot) = Snapshot::take() else {
        // The clipboard is busy or too big to copy; typing leaves it untouched.
        return type_text(text);
    };

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InsertError::Clipboard(e.to_string()))?;
    clipboard
        .set()
        .exclude_from_history()
        .exclude_from_cloud()
        .text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;
    let sent = send_paste();

    thread::sleep(RESTORE_DELAY);
    snapshot.restore();
    sent
}

/// A copy of every memory-backed clipboard format.
struct Snapshot(Vec<(u32, Vec<u8>)>);

/// Holds the clipboard open for as long as it lives.
struct OpenClipboardGuard;

impl OpenClipboardGuard {
    fn open() -> Option<Self> {
        // Another app may be holding the clipboard for a moment; retry briefly.
        for _ in 0..10 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Some(Self);
            }
            thread::sleep(Duration::from_millis(20));
        }
        None
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

impl Snapshot {
    fn take() -> Option<Self> {
        let _open = OpenClipboardGuard::open()?;
        let mut formats = Vec::new();
        let mut total = 0;
        let mut format = 0;
        loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if is_handle_format(format) {
                continue;
            }
            let Some(bytes) = (unsafe { read_global(format) }) else {
                continue;
            };
            total += bytes.len();
            if total > MAX_SNAPSHOT_BYTES {
                return None;
            }
            formats.push((format, bytes));
        }
        Some(Self(formats))
    }

    fn restore(self) {
        let Some(_open) = OpenClipboardGuard::open() else {
            return;
        };
        unsafe {
            let _ = EmptyClipboard();
            for (format, bytes) in self.0 {
                if let Some(handle) = write_global(&bytes) {
                    // On success the system owns the memory.
                    let _ = SetClipboardData(format, Some(HANDLE(handle.0)));
                }
            }
        }
    }
}

/// Copies one clipboard format's memory. The clipboard must be open.
unsafe fn read_global(format: u32) -> Option<Vec<u8>> {
    let handle = GetClipboardData(format).ok()?;
    let global = HGLOBAL(handle.0);
    let size = GlobalSize(global);
    let data = GlobalLock(global) as *const u8;
    if data.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(data, size).to_vec();
    let _ = GlobalUnlock(global);
    Some(bytes)
}

unsafe fn write_global(bytes: &[u8]) -> Option<HGLOBAL> {
    let global = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).ok()?;
    let data = GlobalLock(global) as *mut u8;
    if data.is_null() {
        return None;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
    let _ = GlobalUnlock(global);
    Some(global)
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

fn send_paste() -> Result<(), InsertError> {
    send(&[
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ])
}

/// Types `text` as Unicode keystrokes. Line breaks are sent as Enter.
fn type_text(text: &str) -> Result<(), InsertError> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for c in text.chars() {
        match c {
            '\r' => continue,
            '\n' => {
                inputs.push(key(VK_RETURN, false));
                inputs.push(key(VK_RETURN, true));
            }
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    let down = keyboard_input(VIRTUAL_KEY(0), *unit, KEYEVENTF_UNICODE);
                    let up =
                        keyboard_input(VIRTUAL_KEY(0), *unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
                    inputs.extend([down, up]);
                }
            }
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
