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
    history::{History, Record},
    hotkey::{self, HotkeyEvent},
    insert,
    log::log,
    overlay::{self, Phase},
    problem,
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
/// Recording length before transcribing ahead starts. Shorter dictations are
/// transcribed in one piece on release, which is the most accurate.
const AHEAD_START_SECONDS: f32 = 8.0;
/// Shortest part transcribed ahead. Longer parts give Whisper whole sentences.
const AHEAD_MIN_SECONDS: f32 = 6.0;
/// Audio at the end that's never transcribed ahead: the user may be mid-word.
const AHEAD_TAIL_SECONDS: f32 = 0.6;
const DONE_LINGER: Duration = Duration::from_millis(600);
/// Long enough to read the short reason on the pill.
const ERROR_LINGER: Duration = Duration::from_millis(4000);
/// Longest a finished dictation waits for the speech model to load, e.g. when
/// the user dictates right after start-up. Large Turbo can take a minute or
/// more on a slow laptop, and giving up loses the recording, so this is only
/// a guard against a load that never ends.
const LOAD_WAIT: Duration = Duration::from_secs(5 * 60);
/// Lets the tray menu close and focus settle before "Paste last dictation"
/// looks at which window has focus.
const MENU_SETTLE: Duration = Duration::from_millis(200);

pub fn start(app: AppHandle) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let talk_key = app.state::<Store>().get().talk_key;
    simulate_talk_key(&app, &tx);
    hotkey::start(tx, &talk_key)?;

    let (finish_tx, finish_rx) = mpsc::channel();
    let (hide_tx, hide_rx) = mpsc::channel();
    let shared = app.state::<Shared>();
    let recording = shared.recording.clone();
    *lock(&shared.hide) = Some(hide_tx.clone());
    let pending = Pending::default();
    let finisher = Finisher {
        app: app.clone(),
        hide: hide_tx,
        recording: recording.clone(),
        pending: pending.clone(),
    };
    std::thread::Builder::new()
        .name("dictation-finish".into())
        .spawn(move || finisher.run(finish_rx))?;
    std::thread::Builder::new()
        .name("dictation".into())
        .spawn(move || run(app, rx, finish_tx, hide_rx, recording, pending))?;
    Ok(())
}

/// State shared with "Paste last dictation". Managed by Tauri, so the tray
/// menu and the Settings window can reach it.
#[derive(Default)]
pub struct Shared {
    /// The last dictation's text, kept in memory even when history is off,
    /// so a dictation that couldn't be typed can still be pasted.
    last: Mutex<Option<String>>,
    /// Tells the dictation thread when to hide the overlay.
    hide: Mutex<Option<Sender<Instant>>>,
    /// Set while the talk key is held.
    recording: Arc<AtomicBool>,
}

impl Shared {
    /// Forgets the last dictation, e.g. when the user clears history.
    pub fn forget(&self) {
        *lock(&self.last) = None;
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// An error from a dictation that finished while the next one was being
/// recorded. It's shown once that recording ends, instead of covering the
/// "listening" indicator.
type Pending = Arc<Mutex<Option<String>>>;

/// Text transcribed ahead while the key is still held.
#[derive(Default)]
struct Ahead {
    /// Samples already transcribed, from the start of the recording.
    done: usize,
    texts: Vec<String>,
    /// The language Whisper heard in the first part, for cleanup.
    language: Option<&'static str>,
    /// Time spent transcribing ahead.
    busy: Duration,
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
    /// Stopped at MAX_RECORDING, not by releasing the key. The user may have
    /// walked away, so the text is copied rather than typed.
    timed_out: bool,
    /// When the talk key was released (or the time limit hit).
    released: Instant,
}

fn run(
    app: AppHandle,
    rx: Receiver<HotkeyEvent>,
    finish: Sender<Finished>,
    hide_rx: Receiver<Instant>,
    recording: Arc<AtomicBool>,
    pending: Pending,
) {
    let mut active: Option<Active> = None;
    let mut hide_at: Option<Instant> = None;

    loop {
        let event = match rx.recv_timeout(LEVEL_INTERVAL) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        // Catches a talk key let go without a key-up; its release arrives
        // as a normal event on a later turn.
        hotkey::watchdog();
        match (event, active.take()) {
            (Some(HotkeyEvent::Pressed), None) => {
                hide_at = None;
                let settings = app.state::<Store>().get();
                let opening = Instant::now();
                let started = Recording::start(settings.microphone.as_deref());
                log!("microphone opened in {} ms", opening.elapsed().as_millis());
                match started {
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
                    hide_at = show_pending_or_hide(&app, &pending);
                } else {
                    overlay::set_status(&app, Phase::Transcribing, None);
                    let _ = finish.send(Finished {
                        samples: rec.rec.finish(),
                        ahead: rec.ahead,
                        job: rec.job,
                        timed_out: false,
                        released: Instant::now(),
                    });
                }
            }
            // A shortcut like Right Ctrl+C while holding: drop the audio quietly.
            (Some(HotkeyEvent::Interrupted), Some(_)) => {
                recording.store(false, Ordering::Relaxed);
                hide_at = show_pending_or_hide(&app, &pending);
            }
            (_, current) => active = current,
        }

        if let Some(rec) = &mut active {
            if rec.started.elapsed() >= MAX_RECORDING {
                let rec = active.take().expect("recording is active");
                recording.store(false, Ordering::Relaxed);
                log!(
                    "recording stopped at the {} s limit",
                    MAX_RECORDING.as_secs()
                );
                overlay::set_status(&app, Phase::Transcribing, None);
                let _ = finish.send(Finished {
                    samples: rec.rec.finish(),
                    ahead: rec.ahead,
                    job: rec.job,
                    timed_out: true,
                    released: Instant::now(),
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
    // Checked against the time recorded first, so short dictations never
    // copy the audio.
    let recorded = seconds_to_samples(rec.started.elapsed().as_secs_f32());
    let done = rec.ahead.lock().unwrap().done;
    if !worth_transcribing_ahead(recorded, done) {
        return;
    }
    let samples = rec.rec.snapshot();
    if !worth_transcribing_ahead(samples.len(), done) {
        return;
    }
    let min = seconds_to_samples(AHEAD_MIN_SECONDS);

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
            let started = Instant::now();
            if let Ok(heard) = engine.transcribe(&pending[..cut], &options(&settings)) {
                let mut ahead = ahead.lock().unwrap();
                ahead.busy += started.elapsed();
                ahead.done = done + cut;
                if !heard.text.is_empty() {
                    ahead.texts.push(heard.text);
                    ahead.language = ahead.language.or(heard.language);
                }
            }
        })
        .ok();
}

/// Whether `recorded` samples, of which `done` are transcribed, are enough
/// to transcribe a part ahead.
fn worth_transcribing_ahead(recorded: usize, done: usize) -> bool {
    recorded >= seconds_to_samples(AHEAD_START_SECONDS)
        && recorded.saturating_sub(done) >= seconds_to_samples(AHEAD_MIN_SECONDS)
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
    pending: Pending,
}

impl Finisher {
    fn run(self, rx: Receiver<Finished>) {
        for finished in rx {
            let typed = match self.finish(finished) {
                Ok(typed) => Some(typed),
                Err(message) => {
                    // Logged and reported right away, even while the next
                    // dictation is being recorded, so no failure goes unseen.
                    log!("{message}");
                    problem::report(&self.app, &message);
                    *self.pending.lock().unwrap() = Some(message);
                    None
                }
            };
            // Stored before this check, so if the recording ends meanwhile
            // either this or the dictation thread sees the error.
            if self.recording.load(Ordering::Relaxed) {
                continue;
            }
            let pending = self.pending.lock().unwrap().take();
            let (phase, message, linger) = match (pending, typed) {
                // An earlier failure is shown over this one's success: the
                // text this one typed is already visible in the app.
                (Some(message), _) => (Phase::Error, Some(message), ERROR_LINGER),
                (None, Some(true)) => (Phase::Done, None, DONE_LINGER),
                (None, Some(false)) => (Phase::Empty, None, DONE_LINGER),
                // The dictation thread has already shown this error.
                (None, None) => continue,
            };
            overlay::set_status(&self.app, phase, message);
            let _ = self.hide.send(Instant::now() + linger);
        }
    }

    /// Shows the dictation's progress, unless the next one is being
    /// recorded: its "listening" indicator matters more.
    fn show_progress(&self, phase: Phase, message: Option<String>) {
        if !self.recording.load(Ordering::Relaxed) {
            overlay::set_status(&self.app, phase, message);
        }
    }

    /// Returns whether any text was inserted. Text that couldn't be typed is
    /// copied, and the error says so.
    fn finish(&self, finished: Finished) -> Result<bool, String> {
        let app = &self.app;
        let released = finished.released;
        let joining = Instant::now();
        if let Some(job) = finished.job {
            let _ = job.join();
        }
        let waited = joining.elapsed();
        let settings = app.state::<Store>().get();
        let engine = app.state::<Engine>();
        let (done, mut parts, mut heard_language, busy_ahead) = {
            let ahead = finished.ahead.lock().unwrap();
            (ahead.done, ahead.texts.clone(), ahead.language, ahead.busy)
        };

        // Right after start-up the model may still be loading: keep the
        // audio and wait for it instead of dropping the dictation.
        if engine.loaded_model().is_none() && engine.is_loading() {
            self.show_progress(Phase::Loading, Some("Loading the speech model…".into()));
            let started = Instant::now();
            engine.wait_for_load(LOAD_WAIT);
            log!(
                "waited {} ms for the speech model to load",
                started.elapsed().as_millis()
            );
            if engine.loaded_model().is_none() && engine.is_loading() {
                return Err("The speech model is still loading, so this recording                             was lost. Try again in a moment."
                    .to_owned());
            }
            self.show_progress(Phase::Transcribing, None);
        }

        let rest = finished.samples.get(done..).unwrap_or_default();
        let rest_started = Instant::now();
        let rest = engine
            .transcribe(rest, &options(&settings))
            .inspect_err(|_| {
                // Settings is where a model is downloaded or picked; not
                // worth opening while one is still loading.
                if engine.loaded_model().is_none() && !engine.is_loading() {
                    crate::show_main_window(app);
                }
            })?;
        let rest_time = rest_started.elapsed();
        if !rest.text.is_empty() {
            parts.push(rest.text);
            heard_language = heard_language.or(rest.language);
        }

        let seconds = finished.samples.len() as f32 / TARGET_RATE as f32;
        let peak = finished.samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        // Transcription time per second of audio; above about 0.3 it feels slow.
        let busy = busy_ahead + rest_time;
        let speed = busy.as_secs_f32() / seconds.max(0.1);
        // Whisper reports the language it used (the setting, or the one it
        // detected with "auto"); cleanup rules such as fillers depend on it.
        let language = heard_language.unwrap_or(settings.language.as_str());
        let raw = join_parts(&parts, language);
        let text = cleanup::clean(&raw, settings.cleanup, language, &settings.replacements);
        if text.is_empty() {
            return Ok(false);
        }
        let text_to_insert = with_trailing_space(&text, &settings);

        // Kept before inserting, so a failed insert never loses the text. The
        // history is written on its own thread, starting now, so the text
        // doesn't wait for the disk.
        *lock(&app.state::<Shared>().last) = Some(text_to_insert.clone());
        app.state::<History>().record(
            app,
            Record {
                text,
                seconds,
                speed,
                model: settings
                    .history
                    .then(|| engine.loaded_model().unwrap_or_default().to_owned()),
            },
        );

        let insert_started = Instant::now();
        let inserted = if finished.timed_out {
            Err(NotTyped::TimeLimit)
        } else if test_env("VOCELLE_TEST_NO_INSERT").is_some() {
            eprintln!("vocelle: would insert {text_to_insert:?}");
            Ok(())
        } else {
            insert::insert_text(&text_to_insert).map_err(|e| NotTyped::Failed(e.to_string()))
        };
        log!(
            "dictation: {seconds:.1} s audio, peak {peak:.2}, {} parts, model {}; \
             waited {} ms for the part in progress, last part {} ms, insert {} ms, \
             inserted {} ms after release, {speed:.2} s per audio second",
            parts.len(),
            engine.loaded_model().unwrap_or("none"),
            waited.as_millis(),
            rest_time.as_millis(),
            insert_started.elapsed().as_millis(),
            released.elapsed().as_millis()
        );
        match inserted {
            Ok(()) => Ok(true),
            Err(why) => Err(copy_instead(&text_to_insert, &why)),
        }
    }
}

/// Why a dictation's text wasn't typed.
enum NotTyped {
    /// Typing or pasting failed, with the reason.
    Failed(String),
    /// The recording ran to MAX_RECORDING.
    TimeLimit,
}

/// Copies text that wasn't typed and returns the message for the user.
fn copy_instead(text: &str, why: &NotTyped) -> String {
    let copied = insert::copy_text(text).map_err(|e| e.to_string());
    not_typed_message(why, copied.as_ref().map_err(String::as_str).copied())
}

/// What to tell the user when the text wasn't typed. The first clause is
/// what the overlay shows, so it says where the text is.
fn not_typed_message(why: &NotTyped, copied: Result<(), &str>) -> String {
    let minutes = MAX_RECORDING.as_secs() / 60;
    match (why, copied) {
        (NotTyped::Failed(reason), Ok(())) => format!(
            "Couldn't type here, so your text is copied. Press Ctrl+V to paste it. Reason: {reason}"
        ),
        (NotTyped::TimeLimit, Ok(())) => format!(
            "Text copied after {minutes} minutes. Press Ctrl+V to paste it. \
             Recording stops at {minutes} minutes in case the talk key gets stuck."
        ),
        (NotTyped::Failed(reason), Err(copy)) => format!(
            "Couldn't type or copy your text. Use Paste last dictation in the tray menu. \
             Reason: {reason}; {copy}"
        ),
        (NotTyped::TimeLimit, Err(copy)) => format!(
            "Recording stopped after {minutes} minutes and couldn't be copied. \
             Use Paste last dictation in the tray menu. Reason: {copy}"
        ),
    }
}

/// "Paste last dictation", run off the calling thread because pasting waits
/// for the target app. It pastes only when another app has focus; from the
/// tray menu or the Settings window focus is usually Vocelle's own (or the
/// taskbar's), so it copies the text and asks the user to press Ctrl+V.
pub fn paste_last(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("paste-last".into())
        .spawn(move || {
            std::thread::sleep(MENU_SETTLE);
            paste_last_now(&app);
        });
}

fn paste_last_now(app: &AppHandle) {
    let shared = app.state::<Shared>();
    let Some(text) = lock(&shared.last).clone() else {
        // An error pill, so the reason is shown on screen.
        notify(
            app,
            Phase::Error,
            "No dictation to paste yet.",
            ERROR_LINGER,
        );
        return;
    };
    let recording = shared.recording.load(Ordering::Relaxed);
    if !recording && insert::focus_is_elsewhere() {
        match insert::insert_text(&text) {
            Ok(()) => notify(app, Phase::Done, "Pasted your last dictation.", DONE_LINGER),
            Err(e) => fail(app, copy_instead(&text, &NotTyped::Failed(e.to_string()))),
        }
        return;
    }
    match insert::copy_text(&text) {
        Ok(()) => notify(
            app,
            Phase::Copied,
            "Copied, press Ctrl+V to paste your last dictation.",
            ERROR_LINGER,
        ),
        Err(e) => fail(app, format!("Couldn't copy your last dictation: {e}")),
    }
}

fn fail(app: &AppHandle, message: String) {
    log!("{message}");
    problem::report(app, &message);
    notify(app, Phase::Error, &message, ERROR_LINGER);
}

/// Shows a result on the overlay, unless the user is recording: the
/// "listening" indicator matters more.
fn notify(app: &AppHandle, phase: Phase, message: &str, linger: Duration) {
    let shared = app.state::<Shared>();
    if shared.recording.load(Ordering::Relaxed) {
        return;
    }
    overlay::set_status(app, phase, Some(message.to_owned()));
    overlay::show(app);
    if let Some(hide) = lock(&shared.hide).as_ref() {
        let _ = hide.send(Instant::now() + linger);
    };
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
/// continues it, so its first letter is lowercased ("I" and "I'm" stay, and
/// so do German nouns).
fn join_parts(parts: &[String], language: &str) -> String {
    let lowercase = !cleanup::capitalizes_nouns(language);
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
            Some(c) if continues && lowercase && !keep_capital => {
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
    log!("{message}");
    problem::report(app, &message);
    overlay::set_status(app, Phase::Error, Some(message));
    overlay::show(app);
    Instant::now() + ERROR_LINGER
}

/// Ends a recording that won't be transcribed: shows an error left by the
/// previous dictation if there is one, or hides the overlay. Returns when
/// to hide it.
fn show_pending_or_hide(app: &AppHandle, pending: &Pending) -> Option<Instant> {
    match pending.lock().unwrap().take() {
        Some(message) => {
            overlay::set_status(app, Phase::Error, Some(message));
            Some(Instant::now() + ERROR_LINGER)
        }
        None => {
            overlay::hide(app);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_continue_unfinished_sentences() {
        let parts =
            |p: &[&str]| join_parts(&p.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "en");
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
    fn german_parts_keep_their_capitals() {
        let parts = [
            "Morgen kaufen wir das".to_string(),
            "Haus am See.".to_string(),
        ];
        assert_eq!(
            join_parts(&parts, "de"),
            "Morgen kaufen wir das Haus am See."
        );
        assert_eq!(
            join_parts(&parts, "fr"),
            "Morgen kaufen wir das haus am See."
        );
    }

    #[test]
    fn untyped_text_says_where_it_went() {
        let failed = NotTyped::Failed("Couldn't send keystrokes: blocked".into());
        let copied = not_typed_message(&failed, Ok(()));
        assert!(copied.starts_with("Couldn't type here, so your text is copied."));
        assert!(copied.contains("Press Ctrl+V"));
        assert!(copied.ends_with("Reason: Couldn't send keystrokes: blocked"));

        let lost = not_typed_message(&failed, Err("Clipboard error: busy"));
        assert!(lost.starts_with("Couldn't type or copy your text."));
        assert!(lost.contains("Paste last dictation"));
        assert!(lost.contains("Clipboard error: busy"));

        let timed_out = not_typed_message(&NotTyped::TimeLimit, Ok(()));
        assert!(timed_out.starts_with("Text copied after 5 minutes."));
        assert!(timed_out.contains("Press Ctrl+V"));
        assert!(!timed_out.contains("  "));
        let timed_out = not_typed_message(&NotTyped::TimeLimit, Err("Clipboard error: busy"));
        assert!(timed_out.contains("Paste last dictation"));
        assert!(timed_out.ends_with("Reason: Clipboard error: busy"));
    }

    #[test]
    fn ahead_waits_for_enough_new_audio() {
        let s = seconds_to_samples;
        assert!(!worth_transcribing_ahead(s(7.9), 0));
        assert!(worth_transcribing_ahead(s(8.0), 0));
        // 6 s since the last part are needed.
        assert!(!worth_transcribing_ahead(s(12.0), s(7.0)));
        assert!(worth_transcribing_ahead(s(13.0), s(7.0)));
        assert!(!worth_transcribing_ahead(s(5.0), s(9.0)));
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
