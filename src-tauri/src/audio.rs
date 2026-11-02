//! Microphone capture through cpal (WASAPI on Windows, ALSA/PulseAudio on Linux).
//!
//! Records from the chosen or default input device, mixes to mono, and resamples to the
//! 16 kHz mono f32 that Whisper expects. A rolling input level is kept for the
//! overlay meter.

use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};

pub const TARGET_RATE: u32 = 16_000;

/// Names of the microphones currently connected, for the Settings picker.
pub fn input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.map(|d| d.to_string()).collect())
        .unwrap_or_default()
}

/// A live recording. Dropping it without calling [`Recording::finish`] discards the audio.
pub struct Recording {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    device_rate: u32,
    started: Instant,
}

impl Recording {
    /// Opens the named microphone (or the system default if it's `None` or
    /// unplugged) and starts recording immediately.
    pub fn start(device_name: Option<&str>) -> Result<Self, String> {
        let host = cpal::default_host();
        let chosen =
            device_name.and_then(|name| host.input_devices().ok()?.find(|d| d.to_string() == name));
        let device = chosen
            .or_else(|| host.default_input_device())
            .ok_or("No microphone found. Check your sound settings.")?;
        let supported = device
            .default_input_config()
            .map_err(|e| format!("Couldn't read microphone settings: {e}"))?;

        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let channels = config.channels as usize;
        let device_rate = config.sample_rate;

        let samples = Arc::new(Mutex::new(Vec::<f32>::with_capacity(
            device_rate as usize * 30,
        )));
        let level = Arc::new(AtomicU32::new(0));

        let on_error = |err| eprintln!("vocelle: microphone stream error: {err}");
        let stream = match format {
            SampleFormat::F32 => {
                let sink = Sink::new(&samples, &level, channels);
                device.build_input_stream(
                    config,
                    move |data: &[f32], _: &_| sink.push(data.iter().copied()),
                    on_error,
                    None,
                )
            }
            SampleFormat::I16 => {
                let sink = Sink::new(&samples, &level, channels);
                device.build_input_stream(
                    config,
                    move |data: &[i16], _: &_| {
                        sink.push(data.iter().map(|&s| s as f32 / i16::MAX as f32))
                    },
                    on_error,
                    None,
                )
            }
            SampleFormat::U16 => {
                let sink = Sink::new(&samples, &level, channels);
                device.build_input_stream(
                    config,
                    move |data: &[u16], _: &_| {
                        sink.push(data.iter().map(|&s| (s as f32 - 32_768.0) / 32_768.0))
                    },
                    on_error,
                    None,
                )
            }
            other => return Err(format!("Unsupported microphone sample format: {other:?}")),
        }
        .map_err(|e| format!("Couldn't open the microphone: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("Couldn't start the microphone: {e}"))?;

        Ok(Self {
            stream,
            samples,
            level,
            device_rate,
            started: Instant::now(),
        })
    }

    /// Current input level, 0.0–1.0, for the meter.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// The audio so far as 16 kHz mono, while recording continues.
    pub fn snapshot(&self) -> Vec<f32> {
        if let Some(mut samples) = test_audio() {
            // Test audio arrives in real time, like a microphone.
            let spoken = (self.started.elapsed().as_secs_f32() * TARGET_RATE as f32) as usize;
            samples.truncate(spoken);
            return samples;
        }
        let raw = self.samples.lock().unwrap().clone();
        resample_linear(&raw, self.device_rate, TARGET_RATE)
    }

    /// Stops recording and returns 16 kHz mono samples.
    pub fn finish(self) -> Vec<f32> {
        drop(self.stream);
        if let Some(samples) = test_audio() {
            return samples;
        }
        let raw = std::mem::take(&mut *self.samples.lock().unwrap());
        resample_linear(&raw, self.device_rate, TARGET_RATE)
    }
}

/// Debug builds started with `VOCELLE_TEST_AUDIO=clip.wav` (16 kHz mono 16-bit PCM)
/// return that clip instead of the microphone, for end-to-end tests.
fn test_audio() -> Option<Vec<f32>> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let path = std::env::var_os("VOCELLE_TEST_AUDIO")?;
    let bytes = std::fs::read(path).ok()?;
    // Find the "data" chunk; the header before it varies between writers.
    let data = bytes.windows(4).position(|w| w == b"data")? + 8;
    Some(
        bytes
            .get(data..)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b) as f32 / i16::MAX as f32)
            .collect(),
    )
}

/// Shared state the audio callback writes into.
struct Sink {
    samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    channels: usize,
}

impl Sink {
    fn new(samples: &Arc<Mutex<Vec<f32>>>, level: &Arc<AtomicU32>, channels: usize) -> Self {
        Self {
            samples: samples.clone(),
            level: level.clone(),
            channels: channels.max(1),
        }
    }

    /// Mixes interleaved frames down to mono and updates the level meter.
    fn push(&self, interleaved: impl Iterator<Item = f32>) {
        let mut buf = self.samples.lock().unwrap();
        let start = buf.len();
        let mut acc = 0.0;
        let mut n = 0;
        for s in interleaved {
            acc += s;
            n += 1;
            if n == self.channels {
                buf.push(acc / n as f32);
                acc = 0.0;
                n = 0;
            }
        }
        let chunk = &buf[start..];
        if !chunk.is_empty() {
            let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
            // Speech RMS is usually well under 0.3; scale so normal talking fills the meter.
            let level = (rms * 4.0).min(1.0);
            self.level.store(level.to_bits(), Ordering::Relaxed);
        }
    }
}

/// Linear-interpolation resampler. Good enough for speech recognition input;
/// swap for a windowed-sinc resampler if accuracy tests show a difference.
fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx];
            let b = *input.get(idx + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::resample_linear;

    #[test]
    fn resample_keeps_duration() {
        let one_second_48k = vec![0.0; 48_000];
        assert_eq!(
            resample_linear(&one_second_48k, 48_000, 16_000).len(),
            16_000
        );
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let input = vec![0.1, -0.2, 0.3];
        assert_eq!(resample_linear(&input, 16_000, 16_000), input);
    }
}
