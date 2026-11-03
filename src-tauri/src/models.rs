//! Speech model catalog and downloader.
//!
//! Models are never bundled with the installer. They are downloaded on demand
//! from the official whisper.cpp repositories on Hugging Face, checked against
//! a pinned SHA-256, and stored in the app's local data folder.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// English-only models are faster and more accurate for English speech.
    pub english_only: bool,
    pub size_bytes: u64,
    #[serde(skip)]
    pub file: &'static str,
    #[serde(skip)]
    pub url: &'static str,
    #[serde(skip)]
    pub sha256: &'static str,
}

/// Builds a catalog entry for a file in the official whisper.cpp model repo.
macro_rules! whisper_model {
    ($id:expr, $name:expr, $desc:expr, $en:expr, $file:expr, $size:expr, $sha:expr) => {
        ModelInfo {
            id: $id,
            name: $name,
            description: $desc,
            english_only: $en,
            size_bytes: $size,
            file: $file,
            url: concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                $file
            ),
            sha256: $sha,
        }
    };
}

/// Ordered from fastest to most accurate.
pub const CATALOG: &[ModelInfo] = &[
    whisper_model!(
        "tiny.en",
        "Tiny (English)",
        "Fastest, a little less accurate. Best for slow or older PCs.",
        true,
        "ggml-tiny.en.bin",
        77_704_715,
        "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f"
    ),
    whisper_model!(
        "base.en",
        "Base (English)",
        "Accurate and quick on most laptops. The best place to start.",
        true,
        "ggml-base.en.bin",
        147_964_211,
        "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002"
    ),
    whisper_model!(
        "base",
        "Base (multilingual)",
        "Like Base, for 99 languages.",
        false,
        "ggml-base.bin",
        147_951_465,
        "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
    ),
    whisper_model!(
        "small.en-q5_1",
        "Small (English)",
        "More accurate; needs a recent 4+ core CPU.",
        true,
        "ggml-small.en-q5_1.bin",
        190_098_681,
        "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30"
    ),
    whisper_model!(
        "small-q5_1",
        "Small (multilingual)",
        "Like Small, for 99 languages.",
        false,
        "ggml-small-q5_1.bin",
        190_085_487,
        "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb"
    ),
    whisper_model!(
        "large-v3-turbo-q5_0",
        "Large Turbo",
        "Best accuracy, multilingual. Best with a GPU or 8+ core CPU.",
        false,
        "ggml-large-v3-turbo-q5_0.bin",
        574_041_195,
        "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"
    ),
];

/// Silero voice-activity model used by whisper.cpp to skip silence.
pub const VAD: ModelInfo = ModelInfo {
    id: "silero-vad",
    name: "Silero VAD",
    description: "Detects speech so silence isn't transcribed.",
    english_only: false,
    size_bytes: 885_098,
    file: "ggml-silero-v5.1.2.bin",
    url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin",
    sha256: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
};

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    CATALOG.iter().find(|m| m.id == id)
}

/// Picks a default model from the CPU thread count. A quick heuristic until
/// first-run benchmarking exists.
/// Balances speed and accuracy. On a 2-core/4-thread laptop, Base finished
/// 0.5–2.3 s after release on realistic 8–25 s dictations (with transcribing
/// ahead) and was clearly more accurate than Tiny, so Tiny is only the default
/// on 2-thread machines; Home suggests it when dictation is measurably slow.
/// Large Turbo is never the default on a CPU.
pub fn recommended() -> &'static ModelInfo {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let id = match threads {
        0..=2 => "tiny.en",
        3..=11 => "base.en",
        _ => "small.en-q5_1",
    };
    find(id).expect("recommended model is in the catalog")
}

pub fn path(dir: &Path, model: &ModelInfo) -> PathBuf {
    dir.join(model.file)
}

pub fn is_downloaded(dir: &Path, model: &ModelInfo) -> bool {
    fs::metadata(path(dir, model)).is_ok_and(|m| m.len() == model.size_bytes)
}

/// Downloads `model` into `dir`, resuming a partial download if one exists.
/// `progress` is called with (bytes_done, bytes_total). The file only appears
/// under its final name once its SHA-256 matches.
pub fn download(
    dir: &Path,
    model: &ModelInfo,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let dest = path(dir, model);
    if is_downloaded(dir, model) {
        return Ok(dest);
    }
    fs::create_dir_all(dir).map_err(|e| format!("Couldn't create the models folder: {e}"))?;
    let part = dest.with_extension("part");

    let mut have = fs::metadata(&part).map_or(0, |m| m.len());
    if have > model.size_bytes {
        let _ = fs::remove_file(&part);
        have = 0;
    }

    if have < model.size_bytes {
        let mut request = ureq::get(model.url);
        if have > 0 {
            request = request.header("Range", &format!("bytes={have}-"));
        }
        let mut response = request
            .call()
            .map_err(|e| format!("Download failed: {e}"))?;
        // A server that ignores Range sends the whole file again.
        if have > 0 && response.status() != 206 {
            have = 0;
        }
        let mut out = OpenOptions::new()
            .create(true)
            .write(true)
            .append(have > 0)
            .truncate(have == 0)
            .open(&part)
            .map_err(|e| format!("Couldn't write the model file: {e}"))?;

        let mut reader = response.body_mut().as_reader();
        let mut buf = vec![0u8; 256 * 1024];
        let mut done = have;
        let mut last_report = 0;
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("Download interrupted: {e}"))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("Couldn't write the model file: {e}"))?;
            done += n as u64;
            // Report about every 1 MB to keep UI events cheap.
            if done - last_report >= 1 << 20 || done == model.size_bytes {
                progress(done, model.size_bytes);
                last_report = done;
            }
        }
        out.flush().map_err(|e| e.to_string())?;
    }

    let actual = sha256_file(&part)?;
    if !actual.eq_ignore_ascii_case(model.sha256) {
        let _ = fs::remove_file(&part);
        return Err("The downloaded model was corrupted. Please try again.".into());
    }
    fs::rename(&part, &dest).map_err(|e| format!("Couldn't save the model: {e}"))?;
    progress(model.size_bytes, model.size_bytes);
    Ok(dest)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_hashes_look_valid() {
        for (i, m) in CATALOG.iter().chain([&VAD]).enumerate() {
            assert_eq!(m.sha256.len(), 64, "{}", m.id);
            assert!(m.url.ends_with(m.file), "{}", m.id);
            assert!(
                CATALOG.iter().skip(i + 1).all(|o| o.id != m.id),
                "duplicate id {}",
                m.id
            );
        }
    }

    #[test]
    fn recommended_model_exists() {
        assert!(find(recommended().id).is_some());
    }
}
