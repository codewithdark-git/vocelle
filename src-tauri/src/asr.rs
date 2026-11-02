//! Speech recognition with whisper.cpp, running fully on this PC.
//!
//! Dictation is short and latency is what users feel, so the engine is tuned
//! for that: voice detection trims silence before Whisper runs, the encoder
//! only covers the clip, one decoding state is reused, and the model is warmed
//! up as soon as it loads.

use std::{path::Path, sync::Mutex, time::Instant};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    WhisperVadContext, WhisperVadContextParams, WhisperVadParams,
};

use crate::{audio::TARGET_RATE, models::ModelInfo};

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
/// Length of the warm-up clip. Whisper sizes its buffers for the longest clip
/// so far, so warming up with a typical dictation length keeps later ones fast.
const WARM_UP_SECONDS: usize = 10;

/// Per-dictation choices from Settings.
#[derive(Debug, Default)]
pub struct Options {
    /// "auto" or a language code. English-only models always use English.
    pub language: String,
    /// Words to bias recognition toward, from the personal dictionary.
    pub prompt: Option<String>,
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
    loaded: Mutex<Option<Loaded>>,
}

/// Threads for whisper.cpp: every logical core, up to 8. Measured on a
/// 2-core/4-thread laptop, 4 threads were about a third faster than 2.
fn threads() -> i32 {
    std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8) as i32
}

impl Engine {
    /// Loads `model` from `model_path`, replacing any loaded model. `vad_path`
    /// enables Silero voice-activity detection when the VAD model is present.
    pub fn load(
        &self,
        model: &'static ModelInfo,
        model_path: &Path,
        vad_path: Option<&Path>,
    ) -> Result<(), String> {
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
                .inspect_err(|e| eprintln!("vocelle: voice detection unavailable: {e}"))
                .ok()
        });
        let mut loaded = Loaded { model, state, vad };
        // The first run pays for memory allocation and cold caches; do it now
        // instead of during the user's first dictation.
        let silence = vec![0.0; TARGET_RATE as usize * WARM_UP_SECONDS];
        let _ = run_whisper(&mut loaded, &silence, &Options::default());
        eprintln!(
            "vocelle: loaded {} in {} ms",
            model.id,
            started.elapsed().as_millis()
        );
        *self.loaded.lock().unwrap() = Some(loaded);
        Ok(())
    }

    pub fn loaded_model(&self) -> Option<&'static str> {
        self.loaded.lock().unwrap().as_ref().map(|l| l.model.id)
    }

    /// Transcribes 16 kHz mono audio. Returns an empty string when there was no speech.
    pub fn transcribe(&self, samples: &[f32], options: &Options) -> Result<String, String> {
        let mut guard = self.loaded.lock().unwrap();
        let loaded = guard
            .as_mut()
            .ok_or("No speech model yet. Download one in Vocelle settings.")?;

        let started = Instant::now();
        let speech = match &mut loaded.vad {
            Some(vad) => match speech_span(vad, samples) {
                Some((from, to)) => &samples[from..to],
                None => return Ok(String::new()),
            },
            None if rms(samples) < SILENCE_RMS => return Ok(String::new()),
            None => samples,
        };
        let text = run_whisper(loaded, speech, options)?;
        eprintln!(
            "vocelle: transcribed {:.1} s ({:.1} s of speech) in {} ms with {}",
            samples.len() as f32 / TARGET_RATE as f32,
            speech.len() as f32 / TARGET_RATE as f32,
            started.elapsed().as_millis(),
            loaded.model.id
        );
        Ok(text)
    }

    /// Where `samples` can be cut at a pause between phrases, so the part before
    /// it can be transcribed while the user is still talking. The cut is at
    /// least `min` samples in and leaves at least `tail` samples after it, so
    /// a word that's still being spoken is never split. `None` without voice
    /// detection or when there's no such pause yet.
    pub fn pause_point(&self, samples: &[f32], min: usize, tail: usize) -> Option<usize> {
        let mut guard = self.loaded.lock().unwrap();
        let vad = guard.as_mut()?.vad.as_mut()?;
        let segments: Vec<(usize, usize)> = vad
            .segments_from_samples(WhisperVadParams::default(), samples)
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
                eprintln!("vocelle: voice detection failed: {err}");
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
    // Whisper can get stuck repeating a phrase. Its built-in retry catches most
    // of these (and costs nothing when the first pass is fine); this cap stops
    // the rest. Fast speech is under 6 tokens a second.
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
        engine
            .load(model, Path::new(&model_path), vad.as_deref().map(Path::new))
            .unwrap();

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
            let text = engine.transcribe(&samples, &Options::default()).unwrap();
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
