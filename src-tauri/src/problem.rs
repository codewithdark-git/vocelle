//! The last thing that went wrong, for the "Last problem" card on Home.
//!
//! Failures happen while the user is in another app, so the overlay only
//! shows them for a few seconds. The latest one is kept here (in memory only)
//! until the user dismisses it, and each new one is sent to the Settings
//! window as a `dictation-error` event.

use std::{
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Serialize)]
pub struct Problem {
    pub message: String,
    /// What the user can do about it.
    pub hint: &'static str,
    /// When it happened, in milliseconds since 1970.
    pub time: u64,
}

#[derive(Default)]
pub struct Problems {
    last: Mutex<Option<Problem>>,
}

/// Remembers `message` as the last problem and tells the Settings window.
/// It isn't logged here: callers log where the failure happens.
pub fn report(app: &AppHandle, message: &str) {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let problem = Problem {
        message: message.to_owned(),
        hint: hint(message),
        time,
    };
    if let Some(problems) = app.try_state::<Problems>() {
        *problems.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(problem.clone());
    }
    let _ = app.emit("dictation-error", problem);
}

/// A next step for the user, picked from the wording of the message.
fn hint(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("press ctrl+v") {
        "Click where you want the text and press Ctrl+V. Paste last dictation in the tray menu copies it again."
    } else if lower.contains("paste last dictation") {
        "Your text is kept until the next dictation. Use Paste last dictation in the tray menu to try again."
    } else if lower.contains("administrator") {
        "Windows doesn't let Vocelle type into apps run as administrator. Dictate into another app, or run Vocelle as administrator too."
    } else if lower.contains("microphone") {
        "Check that a microphone is plugged in and that apps are allowed to use it in your privacy settings."
    } else if lower.contains("cpu isn't supported") {
        "Vocelle needs a processor with AVX2 (Intel from 2013 or AMD from 2015 onwards). Support for older processors is planned."
    } else if lower.contains("still loading") {
        "The speech model loads for a few seconds after Vocelle starts. Try again in a moment."
    } else if lower.contains("model") {
        "Open Models and download or choose a speech model."
    } else if lower.contains("clipboard") || lower.contains("keystrokes") {
        "Another app may be using the clipboard or blocking typing. Try again in a moment."
    } else if lower.contains("internal error") {
        "Details are in vocelle.log. If it keeps happening, please report it."
    } else {
        "Try again. If it keeps happening, details are in vocelle.log."
    }
}

#[tauri::command]
pub fn last_problem(problems: State<'_, Problems>) -> Option<Problem> {
    problems
        .last
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

#[tauri::command]
pub fn dismiss_problem(problems: State<'_, Problems>) {
    *problems.last.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_match_the_failure() {
        assert!(hint("Can't type into an app running as administrator").contains("administrator"));
        assert!(hint("No microphone found. Check your sound settings.").contains("microphone"));
        assert!(hint("No speech model yet. Download one in Vocelle settings.").contains("Models"));
        assert!(hint("Clipboard error: busy").contains("clipboard"));
        assert!(hint("This CPU isn't supported yet (needs AVX2).").contains("AVX2"));
        assert!(
            hint("The speech model is still loading. Try again in a moment.")
                .contains("after Vocelle starts")
        );
        assert!(hint("Transcription failed: x").starts_with("Try again"));
        // Text that couldn't be typed: say how to get it, whatever the reason.
        assert!(hint(
            "Couldn't type here, so your text is copied. Press Ctrl+V to paste it.              Reason: Can't type into an app running as administrator"
        )
        .starts_with("Click where you want the text"));
        assert!(hint(
            "Couldn't type or copy your text. Use Paste last dictation in the tray menu."
        )
        .starts_with("Your text is kept"));
    }
}
