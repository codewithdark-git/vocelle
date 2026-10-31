//! Hold-to-talk pipeline: key down → record → key up → transcribe → paste.
//!
//! Runs on one background thread that owns the recording, so the audio stream
//! is created and dropped on the same thread.

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

use tauri::AppHandle;

use crate::{
    asr,
    audio::Recording,
    hotkey::{self, HotkeyEvent},
    insert,
    overlay::{self, Phase},
};

/// Presses shorter than this are treated as accidental taps and ignored.
const MIN_HOLD: Duration = Duration::from_millis(300);
/// How often the overlay meter is refreshed while recording.
const LEVEL_INTERVAL: Duration = Duration::from_millis(50);
const DONE_LINGER: Duration = Duration::from_millis(700);
const ERROR_LINGER: Duration = Duration::from_millis(2500);

pub fn start(app: AppHandle) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    hotkey::start(tx)?;
    std::thread::Builder::new()
        .name("dictation".into())
        .spawn(move || run(app, rx))?;
    Ok(())
}

fn run(app: AppHandle, rx: Receiver<HotkeyEvent>) {
    let mut recording: Option<(Recording, Instant)> = None;
    let mut hide_at: Option<Instant> = None;

    loop {
        match rx.recv_timeout(LEVEL_INTERVAL) {
            Ok(HotkeyEvent::Pressed) => {
                hide_at = None;
                match Recording::start() {
                    Ok(rec) => {
                        overlay::set_status(&app, Phase::Listening, None);
                        overlay::show(&app);
                        recording = Some((rec, Instant::now()));
                    }
                    Err(message) => {
                        hide_at = Some(show_error(&app, message));
                    }
                }
            }
            Ok(HotkeyEvent::Interrupted) => {
                // The user pressed a shortcut like Right Ctrl+C; drop the audio quietly.
                if recording.take().is_some() {
                    overlay::hide(&app);
                }
            }
            Ok(HotkeyEvent::Released) => {
                let Some((rec, started)) = recording.take() else { continue };
                if started.elapsed() < MIN_HOLD {
                    drop(rec);
                    overlay::hide(&app);
                    continue;
                }
                hide_at = Some(finish(&app, rec));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some((rec, _)) = &recording {
            overlay::set_level(&app, rec.level());
        }
        if hide_at.is_some_and(|t| Instant::now() >= t) {
            hide_at = None;
            overlay::hide(&app);
        }
    }
}

/// Transcribes and pastes, returning when the overlay should disappear.
fn finish(app: &AppHandle, rec: Recording) -> Instant {
    overlay::set_status(app, Phase::Transcribing, None);
    let samples = rec.finish();

    let text = match asr::transcribe(&samples) {
        Ok(text) => text,
        Err(message) => return show_error(app, message),
    };
    if let Err(err) = insert::insert_text(text.trim()) {
        return show_error(app, err.to_string());
    }

    overlay::set_status(app, Phase::Done, None);
    Instant::now() + DONE_LINGER
}

fn show_error(app: &AppHandle, message: String) -> Instant {
    eprintln!("vocelle: {message}");
    overlay::set_status(app, Phase::Error, Some(message));
    overlay::show(app);
    Instant::now() + ERROR_LINGER
}
