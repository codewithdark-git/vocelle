//! Dictation pipeline: talk key → record → transcribe → clean up → insert.
//!
//! Runs on one background thread that owns the recording, so the audio stream
//! is created and dropped on the same thread.

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};

use crate::{
    asr::{self, Engine},
    audio::{Recording, TARGET_RATE},
    cleanup,
    history::History,
    hotkey::{self, HotkeyEvent},
    insert,
    overlay::{self, Phase},
    settings::{Settings, Store, TalkMode},
};

/// Presses shorter than this are treated as accidental taps and ignored (hold mode).
const MIN_HOLD: Duration = Duration::from_millis(300);
/// Recordings stop automatically after this, in case the key is never released.
const MAX_RECORDING: Duration = Duration::from_secs(5 * 60);
/// How often the overlay meter is refreshed while recording.
const LEVEL_INTERVAL: Duration = Duration::from_millis(50);
const DONE_LINGER: Duration = Duration::from_millis(700);
const ERROR_LINGER: Duration = Duration::from_millis(2500);

pub fn start(app: AppHandle) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let talk_key = app.state::<Store>().get().talk_key;
    hotkey::start(tx, &talk_key)?;
    std::thread::Builder::new()
        .name("dictation".into())
        .spawn(move || run(app, rx))?;
    Ok(())
}

fn run(app: AppHandle, rx: Receiver<HotkeyEvent>) {
    let mut recording: Option<(Recording, Instant)> = None;
    let mut hide_at: Option<Instant> = None;

    loop {
        let event = match rx.recv_timeout(LEVEL_INTERVAL) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        if let Some(event) = event {
            let settings = app.state::<Store>().get();
            match (event, settings.talk_mode, recording.take()) {
                // Start: any press while idle.
                (HotkeyEvent::Pressed, _, None) => {
                    hide_at = None;
                    match Recording::start(settings.microphone.as_deref()) {
                        Ok(rec) => {
                            overlay::set_status(&app, Phase::Listening, None);
                            overlay::show(&app);
                            recording = Some((rec, Instant::now()));
                        }
                        Err(message) => hide_at = Some(show_error(&app, message)),
                    }
                }
                // Stop: second press in toggle mode.
                (HotkeyEvent::Pressed, TalkMode::Toggle, Some((rec, _))) => {
                    hide_at = Some(finish(&app, rec));
                }
                // Stop: release in hold mode, ignoring accidental taps.
                (HotkeyEvent::Released, TalkMode::Hold, Some((rec, started))) => {
                    if started.elapsed() < MIN_HOLD {
                        overlay::hide(&app);
                    } else {
                        hide_at = Some(finish(&app, rec));
                    }
                }
                // A shortcut like Right Ctrl+C while holding: drop the audio quietly.
                (HotkeyEvent::Interrupted, TalkMode::Hold, Some(_)) => overlay::hide(&app),
                // Anything else leaves the recording running.
                (_, _, active) => recording = active,
            }
        }

        if let Some((rec, started)) = &recording {
            if started.elapsed() >= MAX_RECORDING {
                let (rec, _) = recording.take().expect("recording is active");
                hide_at = Some(finish(&app, rec));
            } else {
                overlay::set_level(&app, rec.level());
            }
        }
        if hide_at.is_some_and(|t| Instant::now() >= t) {
            hide_at = None;
            overlay::hide(&app);
        }
    }
}

/// Transcribes, cleans up and inserts, returning when the overlay should disappear.
fn finish(app: &AppHandle, rec: Recording) -> Instant {
    overlay::set_status(app, Phase::Transcribing, None);
    let samples = rec.finish();
    let settings = app.state::<Store>().get();
    let engine = app.state::<Engine>();

    let options = asr::Options {
        language: settings.language.clone(),
        prompt: cleanup::vocabulary_prompt(&settings.dictionary),
    };
    let raw = match engine.transcribe(&samples, &options) {
        Ok(text) => text,
        Err(message) => {
            if engine.loaded_model().is_none() {
                crate::show_main_window(app);
            }
            return show_error(app, message);
        }
    };

    let text = cleanup::clean(&raw, settings.cleanup, &settings.replacements);
    if text.is_empty() {
        overlay::set_status(app, Phase::Done, Some("No speech heard".into()));
        return Instant::now() + DONE_LINGER;
    }
    if let Err(err) = insert::insert_text(&with_trailing_space(&text, &settings)) {
        return show_error(app, err.to_string());
    }

    if settings.history {
        let seconds = samples.len() as f32 / TARGET_RATE as f32;
        let model = engine.loaded_model().unwrap_or_default();
        app.state::<History>().add(app, &text, seconds, model);
    }
    overlay::set_status(app, Phase::Done, None);
    Instant::now() + DONE_LINGER
}

/// Adds a space so the next dictation doesn't run into this one.
fn with_trailing_space(text: &str, settings: &Settings) -> String {
    if settings.trailing_space && !text.ends_with(char::is_whitespace) {
        format!("{text} ")
    } else {
        text.to_owned()
    }
}

fn show_error(app: &AppHandle, message: String) -> Instant {
    eprintln!("vocelle: {message}");
    overlay::set_status(app, Phase::Error, Some(message));
    overlay::show(app);
    Instant::now() + ERROR_LINGER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_space_is_added_once() {
        let on = Settings::default();
        assert_eq!(with_trailing_space("Hello.", &on), "Hello. ");
        assert_eq!(with_trailing_space("Line\n", &on), "Line\n");
        let off = Settings {
            trailing_space: false,
            ..Settings::default()
        };
        assert_eq!(with_trailing_space("Hello.", &off), "Hello.");
    }
}
