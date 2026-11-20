//! Speech model catalog and downloader.
//!
//! Models are never bundled with the installer. They are downloaded on demand
//! from the official whisper.cpp repositories on Hugging Face, checked against
//! a pinned SHA-256, and stored in the app's local data folder.
//!
//! URLs point at a fixed repository commit, not `main`, so an upstream
//! re-upload can't break the pinned hashes. The commits were checked against
//! the Hugging Face API: every file's size and LFS SHA-256 at that commit
//! match the catalog below.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::log::log;

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
            // Commit 5359861c (2024-10-29) of ggerganov/whisper.cpp.
            url: concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/",
                "5359861c739e955e79d9a303bcbc70fb988958b1/",
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
        "Best accuracy, multilingual. Slow unless your CPU has 8+ cores.",
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
    // Commit 9ffd54a1 (2025-11-17) of ggml-org/whisper-vad.
    url: concat!(
        "https://huggingface.co/ggml-org/whisper-vad/resolve/",
        "9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin"
    ),
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

/// Why a download stopped.
#[derive(Debug, PartialEq, Eq)]
pub enum DownloadError {
    /// The user cancelled it. Nothing to report.
    Cancelled,
    /// A reason to show the user.
    Failed(String),
}

/// How one download attempt ended, with a message for the user and the raw
/// error for the log.
#[derive(Debug)]
enum Failure {
    Cancelled,
    /// Network trouble that may pass, so the download is tried again.
    Transient(String, String),
    /// Trying again won't help (disk full, file gone from the server).
    Fatal(String, String),
}

const STALLED: &str = "The download stalled. Check your internet connection and try again.";
const OFFLINE: &str =
    "Couldn't reach the download server. Check your internet connection and try again.";
const INTERRUPTED: &str =
    "The download was interrupted. Check your internet connection and try again.";
const CORRUPTED: &str = "The downloaded model was corrupted. Please try again.";

/// Pauses before the 2nd and 3rd attempt; after the 3rd failure in a row the
/// download gives up.
const BACKOFF: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(6)];

/// An attempt that got at least this far before failing doesn't count
/// towards giving up: the connection works, it just drops now and then.
const PROGRESS_RESETS_RETRIES: u64 = 1 << 20;

/// A connection that sends nothing for this long fails (and is retried)
/// instead of hanging the download forever.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The pause before trying again after `failures` failed attempts in a row,
/// or `None` to give up.
fn retry_delay(failures: u32) -> Option<Duration> {
    BACKOFF.get(failures.checked_sub(1)? as usize).copied()
}

/// Failed attempts in a row after one more failure that received `received`
/// bytes.
fn failures_after(failures: u32, received: u64) -> u32 {
    if received >= PROGRESS_RESETS_RETRIES {
        1
    } else {
        failures + 1
    }
}

fn status_failure(code: u16) -> Failure {
    let detail = format!("http status {code}");
    match code {
        408 | 429 | 500..=599 => Failure::Transient(
            format!("The download server is busy (HTTP {code}). Try again in a few minutes."),
            detail,
        ),
        404 | 410 => Failure::Fatal(
            format!("The model file wasn't found on the download server (HTTP {code})."),
            detail,
        ),
        _ => Failure::Fatal(
            format!("The download server refused the request (HTTP {code})."),
            detail,
        ),
    }
}

/// Sorts a failed request into retry or give up.
fn request_failure(error: ureq::Error) -> Failure {
    let detail = error.to_string();
    match error {
        ureq::Error::StatusCode(code) => status_failure(code),
        ureq::Error::Timeout(_) => Failure::Transient(STALLED.into(), detail),
        ureq::Error::Io(e) => read_failure(&e),
        _ => Failure::Transient(OFFLINE.into(), detail),
    }
}

/// Sorts a failed body read. ureq reports its own timeouts as an
/// `io::Error` wrapping `ureq::Error::Timeout`.
fn read_failure(error: &io::Error) -> Failure {
    let ureq_timeout = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
        .is_some_and(|inner| matches!(inner, ureq::Error::Timeout(_)));
    let message = if ureq_timeout || error.kind() == io::ErrorKind::TimedOut {
        STALLED
    } else {
        INTERRUPTED
    };
    Failure::Transient(message.into(), error.to_string())
}

fn disk_failure(error: io::Error) -> Failure {
    Failure::Fatal(
        format!("Couldn't save the model file: {error}"),
        error.to_string(),
    )
}

/// A hash mismatch after resuming may come from a bad partial file, so it is
/// worth one fresh try; after a full download it won't get better.
fn mismatch_failure(resumed: bool) -> Failure {
    let detail = "sha-256 mismatch".to_owned();
    if resumed {
        Failure::Transient(CORRUPTED.into(), detail)
    } else {
        Failure::Fatal(CORRUPTED.into(), detail)
    }
}

/// Sleeps for `delay`, waking early on cancel. False if cancelled.
fn wait(delay: Duration, cancel: &AtomicBool) -> bool {
    let until = Instant::now() + delay;
    while !cancel.load(Ordering::Relaxed) {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        std::thread::sleep(left.min(Duration::from_millis(100)));
    }
    false
}

/// An HTTP client that gives up on a dead connection instead of waiting
/// forever. There is no limit on the whole body: a big model on a slow line
/// may take a long time, which is fine while bytes keep arriving.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_resolve(Some(Duration::from_secs(15)))
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(STALL_TIMEOUT))
        .timeout_per_read(Some(STALL_TIMEOUT))
        .build()
        .new_agent()
}

fn part_len(part: &Path) -> u64 {
    fs::metadata(part).map_or(0, |m| m.len())
}

/// Downloads `model` into `dir`, resuming a partial download if one exists
/// and retrying network failures with a pause in between. `progress` is
/// called with (bytes_done, bytes_total). Setting `cancel` stops the
/// download soon after (the partial file is kept for a later resume). The
/// file only appears under its final name once its SHA-256 matches.
pub fn download(
    dir: &Path,
    model: &ModelInfo,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, DownloadError> {
    let dest = path(dir, model);
    if is_downloaded(dir, model) {
        return Ok(dest);
    }
    fs::create_dir_all(dir)
        .map_err(|e| DownloadError::Failed(format!("Couldn't create the models folder: {e}")))?;
    let part = dest.with_extension("part");
    let agent = agent();

    let mut failures = 0;
    loop {
        let before = part_len(&part);
        let (message, detail) = match attempt(&agent, model, &part, cancel, &mut progress) {
            Ok(()) => break,
            Err(Failure::Cancelled) => {
                log!("download of {} cancelled", model.id);
                return Err(DownloadError::Cancelled);
            }
            Err(Failure::Fatal(message, detail)) => {
                log!("download of {} failed: {detail}", model.id);
                return Err(DownloadError::Failed(message));
            }
            Err(Failure::Transient(message, detail)) => (message, detail),
        };
        failures = failures_after(failures, part_len(&part).saturating_sub(before));
        let Some(delay) = retry_delay(failures) else {
            log!(
                "download of {} failed {failures} times, giving up: {detail}",
                model.id
            );
            return Err(DownloadError::Failed(message));
        };
        log!(
            "download of {} failed ({detail}), trying again in {} s",
            model.id,
            delay.as_secs()
        );
        if !wait(delay, cancel) {
            log!("download of {} cancelled", model.id);
            return Err(DownloadError::Cancelled);
        }
    }

    fs::rename(&part, &dest)
        .map_err(|e| DownloadError::Failed(format!("Couldn't save the model: {e}")))?;
    progress(model.size_bytes, model.size_bytes);
    Ok(dest)
}

/// One try at completing and verifying `part`. Keeps what it received on any
/// failure except a hash mismatch, so the next attempt resumes.
fn attempt(
    agent: &ureq::Agent,
    model: &ModelInfo,
    part: &Path,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<(), Failure> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Cancelled);
    }
    let mut have = part_len(part);
    if have > model.size_bytes {
        let _ = fs::remove_file(part);
        have = 0;
    }
    // Whether the bytes checked below came from more than one download, so a
    // hash mismatch may be a bad join rather than a bad file.
    let mut resumed = have > 0;

    if have < model.size_bytes {
        progress(have, model.size_bytes);
        let mut request = agent.get(model.url);
        if have > 0 {
            request = request.header("Range", &format!("bytes={have}-"));
        }
        let mut response = match request.call() {
            Ok(response) => response,
            // The partial file doesn't fit the server's copy; start over.
            Err(ureq::Error::StatusCode(416)) => {
                let _ = fs::remove_file(part);
                return Err(Failure::Transient(
                    INTERRUPTED.into(),
                    "http status 416 for the partial file".into(),
                ));
            }
            Err(e) => return Err(request_failure(e)),
        };
        // A server that ignores Range sends the whole file again.
        if have > 0 && response.status() != 206 {
            have = 0;
            resumed = false;
        }
        let mut out = OpenOptions::new()
            .create(true)
            .write(true)
            .append(have > 0)
            .truncate(have == 0)
            .open(part)
            .map_err(disk_failure)?;

        let mut reader = response.body_mut().as_reader();
        let mut buf = vec![0u8; 256 * 1024];
        let mut done = have;
        let mut last_report = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = out.flush();
                return Err(Failure::Cancelled);
            }
            let n = reader.read(&mut buf).map_err(|e| read_failure(&e))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(disk_failure)?;
            done += n as u64;
            // Report about every 1 MB to keep UI events cheap.
            if done - last_report >= 1 << 20 || done == model.size_bytes {
                progress(done, model.size_bytes);
                last_report = done;
            }
        }
        out.flush().map_err(disk_failure)?;
        if done < model.size_bytes {
            return Err(Failure::Transient(
                INTERRUPTED.into(),
                format!("the server closed the connection at {done} bytes"),
            ));
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Cancelled);
    }
    let actual = sha256_file(part).map_err(|e| Failure::Fatal(e.clone(), e))?;
    if !actual.eq_ignore_ascii_case(model.sha256) {
        let _ = fs::remove_file(part);
        return Err(mismatch_failure(resumed));
    }
    Ok(())
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
    fn urls_are_pinned_to_a_commit() {
        for m in CATALOG.iter().chain([&VAD]) {
            assert!(!m.url.contains("/resolve/main/"), "{}", m.id);
        }
    }

    #[test]
    fn retries_twice_with_growing_pauses_then_gives_up() {
        assert_eq!(retry_delay(1), Some(Duration::from_secs(2)));
        assert_eq!(retry_delay(2), Some(Duration::from_secs(6)));
        assert_eq!(retry_delay(3), None);
        assert_eq!(retry_delay(0), None);
    }

    #[test]
    fn an_attempt_that_made_progress_resets_the_count() {
        assert_eq!(failures_after(0, 0), 1);
        assert_eq!(failures_after(2, 1000), 3);
        assert_eq!(failures_after(2, PROGRESS_RESETS_RETRIES), 1);
        // A connection that keeps dropping after a few MB never gives up.
        let mut failures = 0;
        for _ in 0..10 {
            failures = failures_after(failures, 5 << 20);
            assert!(retry_delay(failures).is_some());
        }
    }

    #[test]
    fn server_errors_are_retried_and_missing_files_are_not() {
        for code in [408, 429, 500, 502, 503] {
            assert!(
                matches!(status_failure(code), Failure::Transient(..)),
                "{code}"
            );
        }
        for code in [401, 403, 404, 410] {
            assert!(matches!(status_failure(code), Failure::Fatal(..)), "{code}");
        }
        assert!(matches!(
            request_failure(ureq::Error::HostNotFound),
            Failure::Transient(..)
        ));
    }

    #[test]
    fn stalled_reads_say_so() {
        let timeout = io::Error::new(io::ErrorKind::TimedOut, "slow");
        assert!(matches!(read_failure(&timeout), Failure::Transient(m, _) if m == STALLED));
        let reset = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
        assert!(matches!(read_failure(&reset), Failure::Transient(m, _) if m == INTERRUPTED));
    }

    #[test]
    fn only_a_resumed_hash_mismatch_is_retried() {
        assert!(matches!(mismatch_failure(true), Failure::Transient(..)));
        assert!(matches!(mismatch_failure(false), Failure::Fatal(..)));
    }

    #[test]
    fn wait_stops_early_on_cancel() {
        let cancel = AtomicBool::new(false);
        assert!(wait(Duration::from_millis(10), &cancel));
        cancel.store(true, Ordering::Relaxed);
        let started = Instant::now();
        assert!(!wait(Duration::from_secs(30), &cancel));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_cancelled_download_stops_before_connecting_and_keeps_the_partial_file() {
        let dir = crate::store_io::test_dir("download-cancel");
        let model = ModelInfo {
            id: "test",
            name: "Test",
            description: "",
            english_only: true,
            size_bytes: 10,
            file: "test.bin",
            // Never contacted: the cancel is seen first.
            url: "http://127.0.0.1:9/test.bin",
            sha256: "",
        };
        fs::write(dir.join("test.part"), b"12345").unwrap();
        let cancel = AtomicBool::new(true);
        let result = download(&dir, &model, &cancel, |_, _| {});
        assert_eq!(result, Err(DownloadError::Cancelled));
        assert_eq!(part_len(&dir.join("test.part")), 5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn recommended_model_exists() {
        assert!(find(recommended().id).is_some());
    }
}
