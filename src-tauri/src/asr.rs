//! Speech recognition.
//!
//! Placeholder until whisper.cpp is wired in (Week 3): it reports how much
//! audio it received, so the rest of the pipeline can be tested end to end.

use crate::audio::TARGET_RATE;

pub fn transcribe(samples: &[f32]) -> Result<String, String> {
    let seconds = samples.len() as f32 / TARGET_RATE as f32;
    Ok(format!("[Vocelle test: heard {seconds:.1} s of audio]"))
}
