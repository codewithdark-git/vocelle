//! Speech recognition with whisper.cpp, running fully on this PC.
//!
//! Dictation is short and latency is what users feel, so the engine is tuned
//! for that: voice detection trims silence before Whisper runs, the encoder
//! only covers the clip, one decoding state is reused, and the model is warmed
//! up as soon as it loads.

use std::{
    path::Path,
    sync::{Condvar, Mutex, RwLock},
    time::{Duration, Instant},
};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    WhisperVadContext, WhisperVadContextParams, WhisperVadParams,
};

use crate::{audio::TARGET_RATE, log::log, models::ModelInfo};

/// whisper.cpp warns on clips under one second, so short clips are padded with silence.
const MIN_SAMPLES: usize = TARGET_RATE as usize * 11 / 10;
/// Clips quieter than this (RMS) are treated as silence when no VAD model is available.
const SILENCE_RMS: f32 = 0.004;
/// Whisper's encoder always covers 30 s (1500 frames, 50 per second). Shrinking it to
/// the clip length is the biggest speed-up for short dictation.
const FULL_AUDIO_CTX: usize = 1500;
const AUDIO_CTX_PER_SECOND: f32 = 50.0;
/// Extra encoder context beyond the clip, which avoids clipped last words.
const AUDIO_CTX_MARGIN_SECONDS: f32 = 1.0;
/// Smallest encoder context. Below about 7 s, Whisper tends to repeat a short
/// phrase over and over; 384 frames fixed that in testing at little cost.
const MIN_AUDIO_CTX: usize = 384;
/// Audio kept around the detected speech, so first and last syllables aren't cut.
const SPEECH_PAD_SECONDS: f32 = 0.25;
/// Length of the warm-up clip. Long enough to allocate Whisper's buffers and
/// warm the caches, short enough to stay cheap on a slow or throttled laptop.
const WARM_UP_SECONDS: usize = 3;
/// Silence needed to split a long dictation into parts. Shorter gaps happen
/// inside phrases, and cutting there breaks words ("buy groceries" became
/// "Microceries" in testing).
const MIN_PAUSE_MS: i32 = 500;

/// Per-dictation choices from Settings.
#[derive(Debug, Default)]
pub struct Options {
    /// "auto" or a language code. English-only models always use English.
    pub language: String,
    /// Words to bias recognition toward, from the personal dictionary.
    pub prompt: Option<String>,
}

/// What the speech model heard.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Transcript {
    /// Empty when there was no speech.
    pub text: String,
    /// The language Whisper transcribed in, such as "en": the one set in
    /// Settings, or the detected one with "auto". `None` when Whisper didn't run.
    pub language: Option<&'static str>,
}

struct Loaded {
    model: &'static ModelInfo,
    /// Reused between dictations; creating a state allocates the model's buffers.
    state: WhisperState,
    vad: Option<WhisperVadContext>,
}

/// The loaded model, shared between the dictation thread and settings commands.
#[derive(Default)]
pub struct Engine {
    /// Held for a whole transcription.
    loaded: Mutex<Option<Loaded>>,
    /// The loaded model's id, kept apart from `loaded` so the Settings window
    /// can read it while a long transcription holds that lock.
    loaded_id: RwLock<Option<&'static str>>,
    loads: Mutex<Loads>,
    /// Signalled when the latest load ends.
    load_done: Condvar,
    /// Held while a model is read from disk, so two loads never run (and
    /// fill memory) at once.
    serial: Mutex<()>,
}

/// Which load request may change the model. Each request takes a ticket, and
/// only the latest one counts, so picking models quickly one after another
/// always ends with the last one picked.
#[derive(Debug, Default)]
struct Loads {
    /// The latest ticket handed out.
    latest: u64,
    /// The model the latest request is loading, until it's done.
    loading: Option<&'static str>,
    /// Why the latest load failed, until the next one starts.
    failed: Option<String>,
}

/// A load request, from [`Engine::begin_load`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket(u64);

impl Loads {
    fn begin(&mut self, id: &'static str) -> Ticket {
        self.latest += 1;
        self.loading = Some(id);
        self.failed = None;
        Ticket(self.latest)
    }

    fn is_current(&self, ticket: Ticket) -> bool {
        ticket.0 == self.latest
    }

    /// Ends `ticket`'s load. Returns false, changing nothing, when a newer
    /// request has replaced it.
    fn finish(&mut self, ticket: Ticket, error: Option<String>) -> bool {
        if !self.is_current(ticket) {
            return false;
        }
        self.loading = None;
        self.failed = error;
        true
    }

    /// Why there's no model to transcribe with. Only says to download one
    /// when no model is being loaded and none failed to load.
    fn not_loaded_message(&self) -> String {
        if self.loading.is_some() {
            "The speech model is still loading. Try again in a moment.".to_owned()
        } else if let Some(error) = &self.failed {
            error.clone()
        } else {
            "No speech model yet. Download one in Vocelle settings.".to_owned()
        }
    }
}

/// Threads for whisper.cpp: every logical core, up to 8. Measured on a
/// 2-core/4-thread laptop, 4 threads were about a third faster than 2.
fn threads() -> i32 {
    std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8) as i32
}

/// CPU features whisper.cpp is built for (see `.cargo/config.toml`). Running
/// it without them crashes the app with an illegal instruction.
const REQUIRED_CPU: [&str; 4] = ["avx", "avx2", "fma", "f16c"];

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn cpu_has(feature: &str) -> bool {
    match feature {
        "avx" => std::is_x86_feature_detected!("avx"),
        "avx2" => std::is_x86_feature_detected!("avx2"),
        "fma" => std::is_x86_feature_detected!("fma"),
        "f16c" => std::is_x86_feature_detected!("f16c"),
        _ => true,
    }
}

/// The x86 baseline doesn't apply to other CPUs.
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn cpu_has(_feature: &str) -> bool {
    true
}

/// The required features `has` says are missing.
fn missing_cpu_features(has: impl Fn(&str) -> bool) -> Vec<&'static str> {
    REQUIRED_CPU.into_iter().filter(|f| !has(f)).collect()
}

/// What to tell the user when the CPU lacks `missing` features.
fn unsupported_cpu_message(missing: &[&str]) -> Option<String> {
    if missing.is_empty() {
        return None;
    }
    let names: Vec<String> = missing.iter().map(|f| f.to_uppercase()).collect();
    Some(format!(
        "This CPU isn't supported yet (needs {}).",
        names.join(", ")
    ))
}

/// Why this CPU can't run the speech engine, if it can't.
fn cpu_problem() -> Option<String> {
    unsupported_cpu_message(&missing_cpu_features(cpu_has))
}

impl Engine {
    /// Starts a load request for `model`. Dictations wait for it (see
    /// [`Engine::wait_for_load`]), and any earlier request that hasn't
    /// finished yet won't replace the model any more.
    pub fn begin_load(&self, id: &'static str) -> Ticket {
        self.loads.lock().unwrap().begin(id)
    }

    /// Ends `ticket`'s request without loading anything, e.g. when the model
    /// file is missing. Returns the error if the request was still current.
    pub fn fail_load(&self, ticket: Ticket, error: String) -> Result<bool, String> {
        self.end_load(ticket, Err(error))
    }

    /// Loads `model` from `model_path` for `ticket`, replacing any loaded
    /// model. `vad_path` enables Silero voice-activity detection when the VAD
    /// model is present. Returns `Ok(false)` when a newer request replaced
    /// this one, which leaves the model as it is.
    pub fn load(
        &self,
        ticket: Ticket,
        model: &'static ModelInfo,
        model_path: &Path,
        vad_path: Option<&Path>,
    ) -> Result<bool, String> {
        let result = self.read_model(ticket, model, model_path, vad_path);
        self.end_load(ticket, result)
    }

    /// Reads and warms up the model. `Ok(None)` when the request was replaced
    /// before it started.
    fn read_model(
        &self,
        ticket: Ticket,
        model: &'static ModelInfo,
        model_path: &Path,
        vad_path: Option<&Path>,
    ) -> Result<Option<Loaded>, String> {
        let _serial = self.serial.lock().unwrap();
        if !self.loads.lock().unwrap().is_current(ticket) {
            log!("skipped loading {}: another model was picked", model.id);
            return Ok(None);
        }
        // Checked first: on a CPU without these, whisper.cpp crashes the app.
        if let Some(problem) = cpu_problem() {
            return Err(problem);
        }
        whisper_rs::install_logging_hooks();
        let started = Instant::now();
        let mut params = WhisperContextParameters::default();
        params.flash_attn(true);
        let ctx = WhisperContext::new_with_params(model_path, params)
            .map_err(|e| format!("Couldn't load the {} model: {e}", model.name))?;
        // The state keeps the model alive, so the context itself can go.
        let state = ctx
            .create_state()
            .map_err(|e| format!("Speech engine error: {e}"))?;
        let vad = vad_path.and_then(|path| {
            let mut params = WhisperVadContextParams::new();
            params.set_n_threads(1);
            params.set_use_gpu(false);
            WhisperVadContext::new(&path.to_string_lossy(), params)
                .inspect_err(|e| log!("voice detection unavailable: {e}"))
                .ok()
        });
        let mut loaded = Loaded { model, state, vad };
        // The first run pays for memory allocation and cold caches; do it now
        // instead of during the user's first dictation.
        let silence = vec![0.0; TARGET_RATE as usize * WARM_UP_SECONDS];
        let _ = run_whisper(&mut loaded, &silence, &Options::default());
        log!(
            "loaded {} in {} ms ({} threads)",
            model.id,
            started.elapsed().as_millis(),
            threads()
        );
        Ok(Some(loaded))
    }

    /// Puts a loaded model in place if `ticket` is still the latest request,
    /// and wakes dictations waiting for it.
    fn end_load(
        &self,
        ticket: Ticket,
        result: Result<Option<Loaded>, String>,
    ) -> Result<bool, String> {
        let outcome = match result {
            Ok(None) => Ok(false),
            Ok(Some(loaded)) => {
                // Waits for a transcription in progress to finish.
                let mut slot = self.loaded.lock().unwrap();
                let mut loads = self.loads.lock().unwrap();
                if loads.finish(ticket, None) {
                    *self.loaded_id.write().unwrap() = Some(loaded.model.id);
                    *slot = Some(loaded);
                    Ok(true)
                } else {
                    log!("dropped {}: another model was picked", loaded.model.id);
                    Ok(false)
                }
            }
            Err(error) => {
                if self
                    .loads
                    .lock()
                    .unwrap()
                    .finish(ticket, Some(error.clone()))
                {
                    Err(error)
                } else {
                    Ok(false)
                }
            }
        };
        self.load_done.notify_all();
        outcome
    }

    pub fn loaded_model(&self) -> Option<&'static str> {
        *self.loaded_id.read().unwrap()
    }

    /// Whether a model is being loaded (at start-up, or after picking one).
    pub fn is_loading(&self) -> bool {
        self.loads.lock().unwrap().loading.is_some()
    }

    /// Waits up to `timeout` for the model being loaded, if any.
    pub fn wait_for_load(&self, timeout: Duration) {
        let loads = self.loads.lock().unwrap();
        let _ = self
            .load_done
            .wait_timeout_while(loads, timeout, |l| l.loading.is_some())
            .unwrap();
    }

    /// Transcribes 16 kHz mono audio. The text is empty when there was no speech.
    pub fn transcribe(&self, samples: &[f32], options: &Options) -> Result<Transcript, String> {
        let mut guard = self.loaded.lock().unwrap();
        let Some(loaded) = guard.as_mut() else {
            return Err(self.loads.lock().unwrap().not_loaded_message());
        };

        let started = Instant::now();
        let speech = match &mut loaded.vad {
            Some(vad) => match speech_span(vad, samples) {
                Some((from, to)) => &samples[from..to],
                None => return Ok(Transcript::default()),
            },
            None if rms(samples) < SILENCE_RMS => return Ok(Transcript::default()),
            None => samples,
        };
        let text = run_whisper(loaded, speech, options)?;
        let language = whisper_rs::get_lang_str(loaded.state.full_lang_id_from_state());
        log!(
            "transcribed {:.1} s ({:.1} s of speech) in {} ms with {} ({})",
            samples.len() as f32 / TARGET_RATE as f32,
            speech.len() as f32 / TARGET_RATE as f32,
            started.elapsed().as_millis(),
            loaded.model.id,
            language.unwrap_or("unknown language")
        );
        Ok(Transcript { text, language })
    }

    /// Where `samples` can be cut at a pause between phrases, so the part before
    /// it can be transcribed while the user is still talking. The cut is at
    /// least `min` samples in and leaves at least `tail` samples after it, so
    /// a word that's still being spoken is never split. `None` without voice
    /// detection or when there's no such pause yet.
    pub fn pause_point(&self, samples: &[f32], min: usize, tail: usize) -> Option<usize> {
        let mut guard = self.loaded.lock().unwrap();
        let vad = guard.as_mut()?.vad.as_mut()?;
        let mut params = WhisperVadParams::default();
        params.set_min_silence_duration(MIN_PAUSE_MS);
        let segments: Vec<(usize, usize)> = vad
            .segments_from_samples(params, samples)
            .ok()?
            .map(|s| (centis_to_index(s.start), centis_to_index(s.end)))
            .collect();
        let limit = samples.len().saturating_sub(tail);
        let pad = (SPEECH_PAD_SECONDS * TARGET_RATE as f32) as usize;
        // The last phrase that ended early enough; cut in the silence after it.
        segments
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, &(_, end))| {
                let next_start = segments.get(i + 1).map_or(samples.len(), |s| s.0);
                let cut = (end + pad).min(next_start).min(limit);
                (end <= limit && cut >= min).then_some(cut)
            })
    }
}

fn centis_to_index(centis: f32) -> usize {
    (centis.max(0.0) / 100.0 * TARGET_RATE as f32) as usize
}

/// The part of `samples` from the first detected speech to the last, padded a
/// little. Pauses in between are kept so sentences stay intact. `None` when
/// nothing sounds like speech.
fn speech_span(vad: &mut WhisperVadContext, samples: &[f32]) -> Option<(usize, usize)> {
    let segments: Vec<(f32, f32)> =
        match vad.segments_from_samples(WhisperVadParams::default(), samples) {
            Ok(segments) => segments.map(|s| (s.start, s.end)).collect(),
            Err(err) => {
                // Transcribe everything rather than drop the dictation.
                log!("voice detection failed: {err}");
                return Some((0, samples.len()));
            }
        };
    let first = segments.first()?.0;
    let last = segments.last()?.1;
    let pad = (SPEECH_PAD_SECONDS * TARGET_RATE as f32) as usize;
    let from = centis_to_index(first)
        .saturating_sub(pad)
        .min(samples.len());
    let to = centis_to_index(last).saturating_add(pad).min(samples.len());
    (to > from).then_some((from, to))
}

fn run_whisper(loaded: &mut Loaded, speech: &[f32], options: &Options) -> Result<String, String> {
    let mut audio = speech.to_vec();
    if audio.len() < MIN_SAMPLES {
        audio.resize(MIN_SAMPLES, 0.0);
    }

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(threads());
    let language = match (loaded.model.english_only, options.language.as_str()) {
        (true, _) => "en",
        (false, "") => "auto",
        (false, code) => code,
    };
    params.set_language(Some(language));
    if let Some(prompt) = &options.prompt {
        params.set_initial_prompt(prompt);
    }
    params.set_audio_ctx(audio_ctx(audio.len()));
    // No retries at higher temperatures: on unclear audio (accents, a quiet
    // mic) Whisper would decode up to five more times, which took over 100 s
    // on a throttled laptop in testing. Repeated phrases are handled by the
    // encoder floor, this token cap (fast speech is under 6 tokens a second)
    // and drop_repeats().
    params.set_temperature_inc(0.0);
    params.set_max_tokens(max_tokens(speech.len()));
    params.set_no_context(true);
    params.set_no_timestamps(true);
    params.set_suppress_blank(true);
    // Drops non-speech tokens such as "[BLANK_AUDIO]" or "(music)".
    params.set_suppress_nst(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    loaded
        .state
        .full(params, &audio)
        .map_err(|e| format!("Transcription failed: {e}"))?;
    let text = loaded
        .state
        .as_iter()
        .filter_map(|seg| seg.to_str_lossy().ok().map(|s| s.trim().to_owned()))
        .filter(|s| !s.is_empty() && !is_non_speech(s))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(drop_repeats(&text))
}

/// Removes a sentence that Whisper repeated back to back, and a cut-off copy
/// of it at the end ("Go now. Go now. Go n" -> "Go now."). People rarely say
/// the same sentence twice in a row in one dictation; Whisper's loops do.
fn drop_repeats(text: &str) -> String {
    let normalize = |s: &str| s.trim_end_matches(['.', '?', '!', ',']).to_lowercase();
    let mut kept: Vec<&str> = Vec::new();
    for sentence in sentences(text) {
        let repeat = kept.last().is_some_and(|last| {
            let (last, this) = (normalize(last), normalize(sentence));
            last == this || (this.len() >= 4 && last.starts_with(&this))
        });
        if !repeat {
            kept.push(sentence);
        }
    }
    kept.join(" ")
}

/// Splits after ".", "?" or "!" followed by a space, keeping the punctuation.
fn sentences(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text.trim();
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = rest
            .char_indices()
            .find(|&(i, c)| {
                matches!(c, '.' | '?' | '!') && rest[i + c.len_utf8()..].starts_with(' ')
            })
            .map_or(rest.len(), |(i, c)| i + c.len_utf8());
        let (sentence, tail) = rest.split_at(end);
        rest = tail.trim_start();
        Some(sentence)
    })
}

/// Upper bound on tokens for a clip, generous for fast speech.
fn max_tokens(samples: usize) -> i32 {
    const TOKENS_PER_SECOND: f32 = 10.0;
    (samples as f32 / TARGET_RATE as f32 * TOKENS_PER_SECOND) as i32 + 16
}

/// Encoder context for a clip, rounded up to a multiple of 64 and capped at the full 30 s.
fn audio_ctx(samples: usize) -> i32 {
    let seconds = samples as f32 / TARGET_RATE as f32 + AUDIO_CTX_MARGIN_SECONDS;
    let ctx = (seconds * AUDIO_CTX_PER_SECOND).ceil() as usize;
    ctx.div_ceil(64)
        .saturating_mul(64)
        .clamp(MIN_AUDIO_CTX, FULL_AUDIO_CTX) as i32
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Whisper sometimes emits bracketed annotations for silence or noise.
fn is_non_speech(segment: &str) -> bool {
    let s = segment.trim();
    (s.starts_with('[') && s.ends_with(']')) || (s.starts_with('(') && s.ends_with(')'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_latest_load_counts() {
        let mut loads = Loads::default();
        let first = loads.begin("tiny");
        let second = loads.begin("small");
        assert!(!loads.is_current(first));
        assert!(loads.is_current(second));
        // The older request finishing changes nothing.
        assert!(!loads.finish(first, Some("failed".into())));
        assert_eq!(loads.loading, Some("small"));
        assert_eq!(loads.failed, None);
        assert!(loads.finish(second, None));
        assert_eq!(loads.loading, None);
    }

    #[test]
    fn a_failed_load_is_kept_until_the_next_one() {
        let mut loads = Loads::default();
        let ticket = loads.begin("small");
        assert!(loads.finish(
            ticket,
            Some("This CPU isn't supported yet (needs AVX2).".into())
        ));
        assert_eq!(
            loads.not_loaded_message(),
            "This CPU isn't supported yet (needs AVX2)."
        );
        loads.begin("tiny");
        assert_eq!(loads.failed, None);
    }

    #[test]
    fn not_loaded_message_says_why() {
        let mut loads = Loads::default();
        assert!(loads.not_loaded_message().contains("Download one"));
        let ticket = loads.begin("small");
        assert!(loads.not_loaded_message().contains("still loading"));
        loads.finish(ticket, None);
        assert!(loads.not_loaded_message().contains("Download one"));
    }

    #[test]
    fn cpu_check_names_missing_features() {
        assert_eq!(missing_cpu_features(|_| true), Vec::<&str>::new());
        assert_eq!(unsupported_cpu_message(&[]), None);
        let missing = missing_cpu_features(|f| f == "avx");
        assert_eq!(missing, ["avx2", "fma", "f16c"]);
        assert_eq!(
            unsupported_cpu_message(&["avx2"]).as_deref(),
            Some("This CPU isn't supported yet (needs AVX2).")
        );
        assert_eq!(
            unsupported_cpu_message(&missing).as_deref(),
            Some("This CPU isn't supported yet (needs AVX2, FMA, F16C).")
        );
    }

    #[test]
    fn this_cpu_passes_the_check() {
        // The tests themselves run code built for this baseline.
        assert_eq!(cpu_problem(), None);
    }

    #[test]
    fn audio_ctx_scales_with_clip_length() {
        let second = TARGET_RATE as usize;
        assert_eq!(audio_ctx(3 * second), 384); // (3 + 1) s * 50 = 200, raised to the minimum
        assert_eq!(audio_ctx(10 * second), 576); // (10 + 1) s * 50 = 550 -> 576
        assert_eq!(audio_ctx(60 * second), 1500);
    }

    #[test]
    fn repeated_sentences_are_dropped() {
        assert_eq!(
            drop_repeats("Meet at 3. Meet at 3. Meet at 3. Meet at"),
            "Meet at 3."
        );
        assert_eq!(
            drop_repeats("Hi. How are you? Fine."),
            "Hi. How are you? Fine."
        );
        assert_eq!(drop_repeats("No. No. Yes."), "No. Yes.");
        assert_eq!(
            drop_repeats("Version 2.5 is out. Ship it."),
            "Version 2.5 is out. Ship it."
        );
    }

    #[test]
    fn bracketed_annotations_are_not_speech() {
        assert!(is_non_speech("[BLANK_AUDIO]"));
        assert!(is_non_speech(" (wind blowing) "));
        assert!(!is_non_speech("Send the report (draft) today."));
    }

    /// Transcribes WAV files with a real model and prints timings. Run with:
    /// `VOCELLE_TEST_MODEL=path/to/ggml-base.en.bin VOCELLE_TEST_WAV=a.wav;b.wav
    ///  cargo test --release transcribe_wav_files -- --ignored --nocapture`
    /// Set `VOCELLE_TEST_VAD` to the Silero model to include voice detection.
    #[test]
    #[ignore = "needs a downloaded model and WAV files"]
    fn transcribe_wav_files() {
        let model_path = std::env::var("VOCELLE_TEST_MODEL").expect("set VOCELLE_TEST_MODEL");
        let file = Path::new(&model_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let model = crate::models::CATALOG
            .iter()
            .find(|m| m.file == file)
            // Other files (for example quantized variants) are tried with
            // Base (English) settings.
            .or_else(|| crate::models::find("base.en"))
            .expect("base.en is in the catalog");
        let vad = std::env::var("VOCELLE_TEST_VAD").ok();
        let engine = Engine::default();
        let ticket = engine.begin_load(model.id);
        let path = Path::new(&model_path);
        assert!(engine
            .load(ticket, model, path, vad.as_deref().map(Path::new))
            .unwrap());

        let wavs = std::env::var("VOCELLE_TEST_WAV").expect("set VOCELLE_TEST_WAV");
        for wav in wavs.split(';') {
            let mut reader = hound::WavReader::open(wav).unwrap();
            let spec = reader.spec();
            assert_eq!(
                (spec.sample_rate, spec.channels),
                (TARGET_RATE, 1),
                "{wav}: need 16 kHz mono"
            );
            let samples: Vec<f32> = reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / i16::MAX as f32)
                .collect();
            let started = Instant::now();
            let text = engine
                .transcribe(&samples, &Options::default())
                .unwrap()
                .text;
            println!(
                "{wav}: {:.1} s audio -> {} ms
  {text}",
                samples.len() as f32 / TARGET_RATE as f32,
                started.elapsed().as_millis()
            );
            assert!(!text.is_empty(), "{wav}: no text");
        }
    }

    #[test]
    fn transcribe_without_model_is_a_clear_error() {
        let err = Engine::default()
            .transcribe(&[0.0; 16_000], &Options::default())
            .unwrap_err();
        assert!(err.contains("Download"));
    }
}
