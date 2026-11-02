//! Floating status pill shown while dictating. It has no text: each phase
//! has its own shape and animation (see overlay.css).
//!
//! It must never take keyboard focus, or the text would be pasted into the
//! overlay instead of the app the user is typing in. On Windows, Tauri's
//! `show()` activates the window, so it is shown with `SW_SHOWNOACTIVATE` and
//! marked `WS_EX_NOACTIVATE`. On Linux the window is made non-focusable.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
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

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Listening,
    Transcribing,
    Done,
    /// Nothing was said, so nothing was typed.
    Empty,
    Error,
}

#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub message: Option<String>,
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
    let _ = app.emit_to(LABEL, "overlay-status", Status { phase, message });
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
