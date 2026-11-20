//! Linux: simulated keys through enigo (X11, with experimental Wayland support).
//!
//! arboard can only read and write text and images here, so when the
//! clipboard holds something else (files, say) the dictation is typed instead
//! of pasted, leaving the clipboard untouched. The old contents are put back
//! only while the clipboard still holds the dictation's text.

use std::sync::Mutex;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::InsertError;
use crate::{hotkey, log::log};

fn enigo() -> Result<Enigo, InsertError> {
    Enigo::new(&Settings::default()).map_err(|e| InsertError::Keystrokes(e.to_string()))
}

fn keystroke_error(e: enigo::InputError) -> InsertError {
    InsertError::Keystrokes(e.to_string())
}

/// The Clipboard that last set text the user still needs to paste. On X11
/// this app serves the clipboard itself, and with no clipboard manager
/// running, dropping the last Clipboard empties it. Replaced by the next one.
static KEPT: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn keep(clipboard: arboard::Clipboard) {
    *KEPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(clipboard);
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

/// The user's clipboard and the dictation that replaced it. The Clipboard is
/// kept alive until the restore: on X11 this app serves the dictation to the
/// target app, and dropping it can take the dictation off the clipboard.
pub struct Saved {
    clipboard: arboard::Clipboard,
    previous: Previous,
    dictation: String,
}

/// Whether the clipboard still holds `dictation`, i.e. nothing was copied since.
fn holds(clipboard: &mut arboard::Clipboard, dictation: &str) -> bool {
    clipboard.get_text().is_ok_and(|text| text == dictation)
}

/// Pastes `text`. `saved` is a restore still waiting from the previous
/// dictation, if any; on return it's the restore to schedule, if any.
pub fn insert(text: &str, saved: &mut Option<Saved>) -> Result<(), InsertError> {
    // When the clipboard still holds the previous dictation, the user's
    // clipboard saved before that is the one to put back.
    let still_ours = saved
        .take()
        .and_then(|mut prev| holds(&mut prev.clipboard, &prev.dictation).then_some(prev));
    let (mut clipboard, previous) = match still_ours {
        Some(prev) => (prev.clipboard, prev.previous),
        None => {
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
            (clipboard, previous)
        }
    };

    clipboard
        .set_text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;
    // Only after a paste: when Ctrl+V couldn't be sent, the dictation stays on
    // the clipboard so the user can still paste it.
    if let Err(err) = send_paste() {
        keep(clipboard);
        return Err(err);
    }
    *saved = Some(Saved {
        clipboard,
        previous,
        dictation: text.to_owned(),
    });
    Ok(())
}

/// Puts the user's clipboard back, unless something was copied after the
/// dictation.
pub fn restore(saved: Saved) {
    let Saved {
        mut clipboard,
        previous,
        dictation,
    } = saved;
    if !holds(&mut clipboard, &dictation) {
        log!("clipboard changed after the paste, so it was left as is");
        return;
    }
    let _ = match previous {
        Previous::Text(prev) => clipboard.set_text(prev),
        Previous::Image(image) => clipboard.set_image(image),
        Previous::Empty => clipboard.clear(),
    };
    // Keeps serving what was put back.
    keep(clipboard);
}

/// Puts `text` on the clipboard and keeps it there after this returns.
pub fn copy(text: &str) -> Result<(), InsertError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InsertError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text)
        .map_err(|e| InsertError::Clipboard(e.to_string()))?;
    keep(clipboard);
    Ok(())
}

/// Same as `copy`: dictations aren't kept out of clipboard history here.
pub fn copy_plain(text: &str) -> Result<(), InsertError> {
    copy(text)
}

/// There's no reliable way to tell which window has focus (none at all on
/// Wayland), so "Paste last dictation" only copies here.
pub fn focus_is_elsewhere() -> bool {
    false
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
