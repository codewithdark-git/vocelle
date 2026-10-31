//! Speech recognition with whisper.cpp, running fully on this PC.

use std::{path::Path, sync::Mutex, time::Instant};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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

struct Loaded {
    model: &'static ModelInfo,
    ctx: WhisperContext,
    vad_path: Option<String>,
}

/// The loaded model, shared between the dictation thread and settings commands.
#[derive(Default)]
pub struct Engine {
    loaded: Mutex<Option<Loaded>>,
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
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Couldn't load the {} model: {e}", model.name))?;
        eprintln!(
            "vocelle: loaded {} in {} ms",
            model.id,
            started.elapsed().as_millis()
        );
        *self.loaded.lock().unwrap() = Some(Loaded {
            model,
            ctx,
            vad_path: vad_path.map(|p| p.to_string_lossy().into_owned()),
        });
        Ok(())
    }

    pub fn loaded_model(&self) -> Option<&'static str> {
        self.loaded.lock().unwrap().as_ref().map(|l| l.model.id)
    }

    /// Transcribes 16 kHz mono audio. Returns an empty string when there was no speech.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        let guard = self.loaded.lock().unwrap();
        let loaded = guard
            .as_ref()
            .ok_or("No speech model yet. Download one in Vocelle settings.")?;

        if loaded.vad_path.is_none() && rms(samples) < SILENCE_RMS {
            return Ok(String::new());
        }

        let mut audio = samples.to_vec();
        if audio.len() < MIN_SAMPLES {
            audio.resize(MIN_SAMPLES, 0.0);
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
        params.set_n_threads(threads as i32);
        params.set_language(Some(if loaded.model.english_only { "en" } else { "auto" }));
        params.set_audio_ctx(audio_ctx(audio.len()));
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_suppress_blank(true);
        // Drops non-speech tokens such as "[BLANK_AUDIO]" or "(music)".
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        if let Some(vad) = &loaded.vad_path {
            // The path must be set first; whisper-rs panics otherwise.
            params.set_vad_model_path(Some(vad));
            params.enable_vad(true);
        }

        let started = Instant::now();
        let mut state = loaded
            .ctx
            .create_state()
            .map_err(|e| format!("Speech engine error: {e}"))?;
        state
            .full(params, &audio)
            .map_err(|e| format!("Transcription failed: {e}"))?;

        let text = state
            .as_iter()
            .filter_map(|seg| seg.to_str_lossy().ok().map(|s| s.trim().to_owned()))
            .filter(|s| !s.is_empty() && !is_non_speech(s))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!(
            "vocelle: transcribed {:.1} s of audio in {} ms with {}",
            samples.len() as f32 / TARGET_RATE as f32,
            started.elapsed().as_millis(),
            loaded.model.id
        );
        Ok(text)
    }
}

/// Encoder context for a clip, rounded up to a multiple of 64 and capped at the full 30 s.
fn audio_ctx(samples: usize) -> i32 {
    let seconds = samples as f32 / TARGET_RATE as f32 + AUDIO_CTX_MARGIN_SECONDS;
    let ctx = (seconds * AUDIO_CTX_PER_SECOND).ceil() as usize;
    ctx.div_ceil(64).saturating_mul(64).min(FULL_AUDIO_CTX) as i32
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
        assert_eq!(audio_ctx(3 * second), 256); // (3 + 1) s * 50 = 200 -> 256
        assert_eq!(audio_ctx(60 * second), 1500);
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
        let file = Path::new(&model_path).file_name().unwrap().to_str().unwrap();
        let model = crate::models::CATALOG
            .iter()
            .find(|m| m.file == file)
            .expect("model file is in the catalog");
        let vad = std::env::var("VOCELLE_TEST_VAD").ok();
        let engine = Engine::default();
        engine
            .load(model, Path::new(&model_path), vad.as_deref().map(Path::new))
            .unwrap();

        let wavs = std::env::var("VOCELLE_TEST_WAV").expect("set VOCELLE_TEST_WAV");
        for wav in wavs.split(';') {
            let mut reader = hound::WavReader::open(wav).unwrap();
            let spec = reader.spec();
            assert_eq!((spec.sample_rate, spec.channels), (TARGET_RATE, 1), "{wav}: need 16 kHz mono");
            let samples: Vec<f32> = reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / i16::MAX as f32)
                .collect();
            let started = Instant::now();
            let text = engine.transcribe(&samples).unwrap();
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
        let err = Engine::default().transcribe(&[0.0; 16_000]).unwrap_err();
        assert!(err.contains("Download"));
    }
}
