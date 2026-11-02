//! Linux: simulated keys through enigo (X11, with experimental Wayland support).

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::InsertError;
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

/// arboard can't list clipboard formats on Linux. If the clipboard holds
/// something that isn't text (an image, files), reading text fails, and the
/// dictation is typed instead of pasted.
pub fn clipboard_is_text_only(clipboard: &mut arboard::Clipboard) -> bool {
    match clipboard.get_text() {
        Ok(_) => true,
        Err(arboard::Error::ContentNotAvailable) => clipboard.get_image().is_err(),
        Err(_) => false,
    }
}

pub fn send_paste() -> Result<(), InsertError> {
    let mut enigo = enigo()?;
    hotkey::while_injecting(|| {
        enigo
            .key(Key::Control, Direction::Press)
            .and_then(|_| enigo.key(Key::Unicode('v'), Direction::Click))
            .and_then(|_| enigo.key(Key::Control, Direction::Release))
            .map_err(keystroke_error)
    })
}

pub fn type_text(text: &str) -> Result<(), InsertError> {
    let mut enigo = enigo()?;
    hotkey::while_injecting(|| enigo.text(text).map_err(keystroke_error))
}
