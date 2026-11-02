//! Dictation pipeline: talk key → record → transcribe → clean up → insert.
//!
//! The dictation thread owns the recording, so the audio stream is created and
//! dropped on the same thread, and it only ever waits for the talk key. That
//! keeps the key responsive even while an earlier dictation is still being
//! typed.
//!
//! While the key is held, phrases that have ended (found with voice
//! detection) are transcribed in the background, so on release only the last
//! few seconds are left. The rest of the work runs on a separate finishing
//! thread, one dictation at a time, so text is inserted in the order spoken.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
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
    settings::{Settings, Store},
};

/// Presses shorter than this are treated as accidental taps and ignored.
const MIN_HOLD: Duration = Duration::from_millis(250);
/// Recordings stop automatically after this, in case the key is never released.
const MAX_RECORDING: Duration = Duration::from_secs(5 * 60);
/// How often the overlay meter is refreshed while recording.
const LEVEL_INTERVAL: Duration = Duration::from_millis(50);
/// How often to look for a finished phrase to transcribe ahead.
const AHEAD_INTERVAL: Duration = Duration::from_millis(800);
/// Untranscribed audio needed before transcribing ahead. Shorter dictations
/// are fast enough to do in one go on release.
const AHEAD_MIN_SECONDS: f32 = 3.0;
/// Audio at the end that's never transcribed ahead: the user may be mid-word.
const AHEAD_TAIL_SECONDS: f32 = 0.6;
const DONE_LINGER: Duration = Duration::from_millis(600);
const ERROR_LINGER: Duration = Duration::from_millis(2000);

pub fn start(app: AppHandle) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let talk_key = app.state::<Store>().get().talk_key;
    simulate_talk_key(&app, &tx);
    hotkey::start(tx, &talk_key)?;

    let (finish_tx, finish_rx) = mpsc::channel();
    let (hide_tx, hide_rx) = mpsc::channel();
    let recording = Arc::new(AtomicBool::new(false));
    let finisher = Finisher {
        app: app.clone(),
        hide: hide_tx,
        recording: recording.clone(),
    };
    std::thread::Builder::new()
        .name("dictation-finish".into())
        .spawn(move || finisher.run(finish_rx))?;
    std::thread::Builder::new()
        .name("dictation".into())
        .spawn(move || run(app, rx, finish_tx, hide_rx, recording))?;
    Ok(())
}

/// Text transcribed ahead while the key is still held.
#[derive(Default)]
struct Ahead {
    /// Samples already transcribed, from the start of the recording.
    done: usize,
    texts: Vec<String>,
}

struct Active {
    rec: Recording,
    started: Instant,
    last_ahead: Instant,
    ahead: Arc<Mutex<Ahead>>,
    /// The background transcription in progress, if any.
    job: Option<JoinHandle<()>>,
}

/// A finished recording handed to the finishing thread.
struct Finished {
    samples: Vec<f32>,
    ahead: Arc<Mutex<Ahead>>,
    job: Option<JoinHandle<()>>,
}

fn run(
    app: AppHandle,
    rx: Receiver<HotkeyEvent>,
    finish: Sender<Finished>,
    hide_rx: Receiver<Instant>,
    recording: Arc<AtomicBool>,
) {
    let mut active: Option<Active> = None;
    let mut hide_at: Option<Instant> = None;

    loop {
        let event = match rx.recv_timeout(LEVEL_INTERVAL) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        match (event, active.take()) {
            (Some(HotkeyEvent::Pressed), None) => {
                hide_at = None;
                let settings = app.state::<Store>().get();
                match Recording::start(settings.microphone.as_deref()) {
                    Ok(rec) => {
                        recording.store(true, Ordering::Relaxed);
                        overlay::set_status(&app, Phase::Listening, None);
                        overlay::show(&app);
                        active = Some(Active {
                            rec,
                            started: Instant::now(),
                            last_ahead: Instant::now(),
                            ahead: Arc::default(),
                            job: None,
                        });
                    }
                    Err(message) => hide_at = Some(show_error(&app, message)),
                }
            }
            (Some(HotkeyEvent::Released), Some(rec)) => {
                recording.store(false, Ordering::Relaxed);
                if rec.started.elapsed() < MIN_HOLD {
                    overlay::hide(&app);
                } else {
                    overlay::set_status(&app, Phase::Transcribing, None);
                    let _ = finish.send(Finished {
                        samples: rec.rec.finish(),
                        ahead: rec.ahead,
                        job: rec.job,
                    });
                }
            }
            // A shortcut like Right Ctrl+C while holding: drop the audio quietly.
            (Some(HotkeyEvent::Interrupted), Some(_)) => {
                recording.store(false, Ordering::Relaxed);
                overlay::hide(&app);
            }
            (_, current) => active = current,
        }

        if let Some(rec) = &mut active {
            if rec.started.elapsed() >= MAX_RECORDING {
                let rec = active.take().expect("recording is active");
                recording.store(false, Ordering::Relaxed);
                overlay::set_status(&app, Phase::Transcribing, None);
                let _ = finish.send(Finished {
                    samples: rec.rec.finish(),
                    ahead: rec.ahead,
                    job: rec.job,
                });
            } else {
                overlay::set_level(&app, rec.rec.level());
                transcribe_ahead(&app, rec);
            }
        }

        while let Ok(at) = hide_rx.try_recv() {
            hide_at = Some(at);
        }
        if active.is_none() && hide_at.is_some_and(|t| Instant::now() >= t) {
            hide_at = None;
            overlay::hide(&app);
        }
    }
}

/// Starts transcribing the phrases spoken so far, if enough has been said and
/// the previous background job is done.
fn transcribe_ahead(app: &AppHandle, rec: &mut Active) {
    if rec.last_ahead.elapsed() < AHEAD_INTERVAL
        || rec.job.as_ref().is_some_and(|j| !j.is_finished())
    {
        return;
    }
    rec.last_ahead = Instant::now();
    let done = rec.ahead.lock().unwrap().done;
    let samples = rec.rec.snapshot();
    let min = seconds_to_samples(AHEAD_MIN_SECONDS);
    if samples.len().saturating_sub(done) < min {
        return;
    }

    let app = app.clone();
    let ahead = rec.ahead.clone();
    rec.job = std::thread::Builder::new()
        .name("dictation-ahead".into())
        .spawn(move || {
            let engine = app.state::<Engine>();
            let pending = &samples[done..];
            let Some(cut) =
                engine.pause_point(pending, min, seconds_to_samples(AHEAD_TAIL_SECONDS))
            else {
                return;
            };
            let settings = app.state::<Store>().get();
            if let Ok(text) = engine.transcribe(&pending[..cut], &options(&settings)) {
                let mut ahead = ahead.lock().unwrap();
                ahead.done = done + cut;
                if !text.is_empty() {
                    ahead.texts.push(text);
                }
            }
        })
        .ok();
}

fn seconds_to_samples(seconds: f32) -> usize {
    (seconds * TARGET_RATE as f32) as usize
}

/// Whisper options from Settings. Earlier parts of the dictation aren't passed
/// as context: that made Whisper repeat itself.
fn options(settings: &Settings) -> asr::Options {
    asr::Options {
        language: settings.language.clone(),
        prompt: cleanup::vocabulary_prompt(&settings.dictionary),
    }
}

/// Finishes dictations one at a time: waits for any background part,
/// transcribes the rest, cleans up and inserts.
struct Finisher {
    app: AppHandle,
    /// Tells the dictation thread when to hide the overlay.
    hide: Sender<Instant>,
    /// Set while a new dictation is being recorded, so this doesn't replace
    /// its "listening" indicator.
    recording: Arc<AtomicBool>,
}

impl Finisher {
    fn run(self, rx: Receiver<Finished>) {
        for finished in rx {
            let started = Instant::now();
            let outcome = self.finish(finished);
            eprintln!(
                "vocelle: finished in {} ms after release",
                started.elapsed().as_millis()
            );
            if !self.recording.load(Ordering::Relaxed) {
                let (phase, message, linger) = match outcome {
                    Ok(true) => (Phase::Done, None, DONE_LINGER),
                    Ok(false) => (Phase::Empty, None, DONE_LINGER),
                    Err(message) => {
                        eprintln!("vocelle: {message}");
                        (Phase::Error, Some(message), ERROR_LINGER)
                    }
                };
                overlay::set_status(&self.app, phase, message);
                let _ = self.hide.send(Instant::now() + linger);
            }
        }
    }

    /// Returns whether any text was inserted.
    fn finish(&self, finished: Finished) -> Result<bool, String> {
        let app = &self.app;
        if let Some(job) = finished.job {
            let _ = job.join();
        }
        let settings = app.state::<Store>().get();
        let engine = app.state::<Engine>();
        let (done, mut parts) = {
            let ahead = finished.ahead.lock().unwrap();
            (ahead.done, ahead.texts.clone())
        };

        let rest = finished.samples.get(done..).unwrap_or_default();
        let rest_text = engine
            .transcribe(rest, &options(&settings))
            .inspect_err(|_| {
                if engine.loaded_model().is_none() {
                    crate::show_main_window(app);
                }
            })?;
        if !rest_text.is_empty() {
            parts.push(rest_text);
        }

        let raw = join_parts(&parts);
        let text = cleanup::clean(&raw, settings.cleanup, &settings.replacements);
        if text.is_empty() {
            return Ok(false);
        }
        let text_to_insert = with_trailing_space(&text, &settings);
        if test_env("VOCELLE_TEST_NO_INSERT").is_some() {
            eprintln!("vocelle: would insert {text_to_insert:?}");
        } else {
            insert::insert_text(&text_to_insert).map_err(|e| e.to_string())?;
        }

        let history = app.state::<History>();
        let seconds = finished.samples.len() as f32 / TARGET_RATE as f32;
        history.count(app, &text, seconds);
        if settings.history {
            let model = engine.loaded_model().unwrap_or_default();
            history.add(app, &text, seconds, model);
        }
        Ok(true)
    }
}

/// Reads a test setting. Only debug builds honour these; release builds never do.
fn test_env(name: &str) -> Option<String> {
    cfg!(debug_assertions)
        .then(|| std::env::var(name).ok())
        .flatten()
}

/// Debug builds started with `VOCELLE_TEST_DICTATE=<ms>` hold the talk key for
/// that long once the model is ready, without touching the real keyboard.
/// Pair with `VOCELLE_TEST_AUDIO` and `VOCELLE_TEST_NO_INSERT` to test the whole
/// pipeline without typing into whatever app has focus.
fn simulate_talk_key(app: &AppHandle, tx: &Sender<HotkeyEvent>) {
    let Some(hold) = test_env("VOCELLE_TEST_DICTATE").and_then(|ms| ms.parse().ok()) else {
        return;
    };
    let (app, tx) = (app.clone(), tx.clone());
    std::thread::spawn(move || {
        while app.state::<Engine>().loaded_model().is_none() {
            std::thread::sleep(Duration::from_millis(200));
        }
        std::thread::sleep(Duration::from_secs(1));
        let _ = tx.send(HotkeyEvent::Pressed);
        std::thread::sleep(Duration::from_millis(hold));
        let _ = tx.send(HotkeyEvent::Released);
    });
}

/// Joins the parts of one dictation. Whisper starts every part with a
/// capital; when the previous part didn't end a sentence, the next one
/// continues it, so its first letter is lowercased ("I" and "I'm" stay).
fn join_parts(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
        if out.is_empty() {
            out.push_str(part);
            continue;
        }
        let continues = !out.ends_with(['.', '?', '!']);
        let first_word = part.split_whitespace().next().unwrap_or_default();
        let keep_capital = first_word == "I" || first_word.starts_with("I'");
        out.push(' ');
        let mut chars = part.chars();
        match chars.next() {
            Some(c) if continues && !keep_capital => {
                out.extend(c.to_lowercase());
                out.push_str(chars.as_str());
            }
            _ => out.push_str(part),
        }
    }
    out
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
    fn parts_continue_unfinished_sentences() {
        let parts = |p: &[&str]| join_parts(&p.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(
            parts(&["So the launch is", "Ready for Friday."]),
            "So the launch is ready for Friday."
        );
        assert_eq!(
            parts(&["We met.", "Then we left."]),
            "We met. Then we left."
        );
        assert_eq!(parts(&["Yesterday", "I'm done"]), "Yesterday I'm done");
        assert_eq!(parts(&["", "Hello"]), "Hello");
    }

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
