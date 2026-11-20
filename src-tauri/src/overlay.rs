//! Floating status pill shown while dictating. Each phase has its own shape
//! and animation (see overlay.css). Only errors, the "copied" notice and
//! "loading the speech model" add words: the window widens to fit one line, then shrinks back for the next
//! dictation.
//!
//! It must never take keyboard focus, or the text would be pasted into the
//! overlay instead of the app the user is typing in. On Windows, Tauri's
//! `show()` activates the window, so it is shown with `SW_SHOWNOACTIVATE` and
//! marked `WS_EX_NOACTIVATE`. On Linux the window is made non-focusable.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};
#[cfg(windows)]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_SHOWNOACTIVATE,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    },
};

pub const LABEL: &str = "overlay";
/// Gap between the overlay and the bottom of the screen's work area, so it sits
/// just above the taskbar, out of the way of what the user is typing.
const BOTTOM_MARGIN: i32 = 12;
/// Window size, matching `tauri.conf.json`.
const WIDTH: f64 = 160.0;
const HEIGHT: f64 = 64.0;
/// Window width while an error's reason is shown.
const ERROR_WIDTH: f64 = 400.0;
/// Longest reason shown on the pill; the full message is its accessible name.
const SHORT_CHARS: usize = 50;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Listening,
    Transcribing,
    /// The dictation waits for the speech model, which is still loading.
    Loading,
    Done,
    /// Nothing was said, so nothing was typed.
    Empty,
    /// The text is on the clipboard for the user to paste.
    Copied,
    Error,
}

#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub message: Option<String>,
    /// A short form of `message` that fits on the pill.
    pub label: Option<String>,
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

#[cfg(windows)]
fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| HWND(h.0))
}

/// Applies the no-activate window styles. Call once at startup.
pub fn init(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    let _ = window.set_ignore_cursor_events(true);
    #[cfg(windows)]
    if let Some(hwnd) = hwnd(&window) {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let extra = (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | extra);
        }
    }
    #[cfg(not(windows))]
    let _ = window.set_focusable(false);
}

/// Shows the overlay bottom-center on the monitor under the cursor, without taking focus.
pub fn show(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    position(&window);
    #[cfg(windows)]
    if let Some(hwnd) = hwnd(&window) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
    #[cfg(not(windows))]
    let _ = window.show();
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = window(app) {
        let _ = window.hide();
    }
}

pub fn set_status(app: &AppHandle, phase: Phase, message: Option<String>) {
    // Several threads set the status; one resize and its status are sent
    // together, so the window's width always fits the phase shown.
    static SETTING: Mutex<()> = Mutex::new(());
    let _setting = SETTING.lock().unwrap_or_else(|e| e.into_inner());
    let label = message.as_deref().map(short_message);
    let wide = matches!(phase, Phase::Error | Phase::Copied | Phase::Loading) && label.is_some();
    if let Some(window) = window(app) {
        fit(&window, if wide { ERROR_WIDTH } else { WIDTH });
    }
    let _ = app.emit_to(
        LABEL,
        "overlay-status",
        Status {
            phase,
            message,
            label,
        },
    );
}

/// Resizes the window to `width` (logical pixels) and keeps it centered.
/// Resizing doesn't activate it, so focus stays where the user is typing.
fn fit(window: &WebviewWindow, width: f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = LogicalSize::new(width, HEIGHT).to_physical::<u32>(scale);
    if window.inner_size().is_ok_and(|s| s == size) {
        return;
    }
    let _ = window.set_size(size);
    if window.is_visible().unwrap_or(false) {
        position(window);
    }
}

/// The first sentence or clause of `message`, cut to fit on the pill:
/// "Couldn't open the microphone: device busy" -> "Couldn't open the microphone".
fn short_message(message: &str) -> String {
    let message = message.trim();
    let end = message
        .char_indices()
        .find(|&(i, c)| (c == ':' || c == '.') && message[i + 1..].starts_with(char::is_whitespace))
        .map_or(message.len(), |(i, _)| i);
    let first = message[..end].trim_end();
    if first.chars().count() <= SHORT_CHARS {
        return first.to_owned();
    }
    let cut: String = first.chars().take(SHORT_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

pub fn set_level(app: &AppHandle, level: f32) {
    let _ = app.emit_to(LABEL, "overlay-level", level);
}

fn position(window: &WebviewWindow) {
    let monitor = window
        .cursor_position()
        .ok()
        .and_then(|p| window.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - BOTTOM_MARGIN;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_keeps_the_first_clause() {
        assert_eq!(
            short_message("No microphone found. Check your sound settings."),
            "No microphone found"
        );
        assert_eq!(
            short_message("Couldn't open the microphone: device in use"),
            "Couldn't open the microphone"
        );
        assert_eq!(
            short_message("Can't type into an app running as administrator"),
            "Can't type into an app running as administrator"
        );
        // A dot inside a word or number isn't a sentence end.
        assert_eq!(short_message("Model v1.5 failed"), "Model v1.5 failed");
    }

    #[test]
    fn short_message_is_cut_to_fit() {
        let long = "word ".repeat(20);
        let short = short_message(&long);
        assert!(short.chars().count() <= SHORT_CHARS);
        assert!(short.ends_with('…'));
    }
}
