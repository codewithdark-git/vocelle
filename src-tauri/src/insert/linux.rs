//! Linux: simulated keys through enigo (X11, with experimental Wayland support).
//!
//! arboard can only read and write text and images here, so when the
//! clipboard holds something else (files, say) the dictation is typed instead
//! of pasted, leaving the clipboard untouched.

use std::thread;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::{InsertError, RESTORE_DELAY};
use crate::hotkey;

fn enigo() -> Result<Enigo, InsertError> {
    Enigo::new(&Settings::default()).map_err(|e| InsertError::Keystrokes(e.to_string()))
}

fn keystroke_error(e: enigo::InputError) -> InsertError {
    InsertError::Keystrokes(e.to_string())
}

/// Linux has no equivalent of Windows' elevated-window protection for X11 apps.
pub fn check_target() -> Result<(), InsertError> {
    Ok(())
}

enum Previous {
    Empty,
    Text(String),
    Image(arboard::ImageData<'static>),
}

pub fn insert(text: &str) -> Result<(), InsertError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InsertError::Clipboard(e.to_string()))?;
    let previous = match clipboard.get_text() {
        Ok(text) => Previous::Text(text),
        Err(arboard::Error::ContentNotAvailable) => match clipboard.get_image() {
            Ok(image) => Previous::Image(image),
            Err(arboard::Error::ContentNotAvailable) => Previous::Empty,
            Err(_) => return type_text(text),
        },
        Err(_) => return type_text(text),
    };

    clipboard
        .set_text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;
    let sent = send_paste();

    thread::sleep(RESTORE_DELAY);
    let _ = match previous {
        Previous::Text(prev) => clipboard.set_text(prev),
        Previous::Image(image) => clipboard.set_image(image),
        Previous::Empty => clipboard.clear(),
    };
    sent
}

fn send_paste() -> Result<(), InsertError> {
    let mut enigo = enigo()?;
    hotkey::while_injecting(|| {
        enigo
            .key(Key::Control, Direction::Press)
            .and_then(|_| enigo.key(Key::Unicode('v'), Direction::Click))
            .and_then(|_| enigo.key(Key::Control, Direction::Release))
            .map_err(keystroke_error)
    })
}

fn type_text(text: &str) -> Result<(), InsertError> {
    let mut enigo = enigo()?;
    hotkey::while_injecting(|| enigo.text(text).map_err(keystroke_error))
}
