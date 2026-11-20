//! Settings-window commands for speech models, and loading the chosen model.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, TryLockError,
    },
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    asr::{Engine, Ticket},
    log::log,
    models::{self, DownloadError, ModelInfo},
    problem,
    settings::Store,
};

/// Held while a model is saved as the active one and its load request starts,
/// so the latest load request is always for the model in Settings.
static SELECT: Mutex<()> = Mutex::new(());

/// Model ids currently downloading, each with its cancel flag, so a double
/// click doesn't start two downloads. An id leaves the map whenever its
/// download thread ends, however it ends, so Download always works again.
#[derive(Default)]
pub struct Downloads(Mutex<HashMap<&'static str, Arc<AtomicBool>>>);

impl Downloads {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<&'static str, Arc<AtomicBool>>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Removes a download's id from [`Downloads`] when its thread ends, even if
/// it panics.
struct Entry {
    app: AppHandle,
    id: &'static str,
    removed: bool,
}

impl Drop for Entry {
    fn drop(&mut self) {
        if !self.removed {
            self.app.state::<Downloads>().lock().remove(self.id);
        }
    }
}

#[derive(Serialize)]
pub struct ModelStatus {
    #[serde(flatten)]
    info: ModelInfo,
    downloaded: bool,
    downloading: bool,
    selected: bool,
    loaded: bool,
    recommended: bool,
}

#[derive(Clone, Serialize)]
struct Progress {
    id: &'static str,
    done: u64,
    total: u64,
}

#[derive(Clone, Serialize)]
struct DownloadEvent {
    id: &'static str,
    error: Option<String>,
    /// The user cancelled it, so there is no error to show.
    cancelled: bool,
}

#[derive(Clone, Serialize)]
struct ModelEvent {
    id: &'static str,
    error: Option<String>,
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|d| d.join("models"))
        .map_err(|e| format!("No data folder available: {e}"))
}

fn find(id: &str) -> Result<&'static ModelInfo, String> {
    models::find(id).ok_or_else(|| format!("Unknown model: {id}"))
}

/// Async, so it runs off the main thread.
#[tauri::command]
pub async fn list_models(
    app: AppHandle,
    engine: State<'_, Engine>,
    downloads: State<'_, Downloads>,
) -> Result<Vec<ModelStatus>, String> {
    let dir = models_dir(&app)?;
    let selected = app.state::<Store>().get().model;
    let loaded = engine.loaded_model();
    let downloading = downloads.lock();
    let recommended = models::recommended().id;
    Ok(models::CATALOG
        .iter()
        .map(|m| ModelStatus {
            info: *m,
            downloaded: models::is_downloaded(&dir, m),
            downloading: downloading.contains_key(m.id),
            selected: selected.as_deref() == Some(m.id),
            loaded: loaded == Some(m.id),
            recommended: m.id == recommended,
        })
        .collect())
}

/// Starts downloading a model (and the small VAD model) in the background.
/// Progress arrives as `model-progress` events, then `model-downloaded`.
#[tauri::command]
pub fn download_model(
    app: AppHandle,
    downloads: State<'_, Downloads>,
    id: String,
) -> Result<(), String> {
    let model = find(&id)?;
    let dir = models_dir(&app)?;
    let cancel = {
        let mut map = downloads.lock();
        if let Some(cancel) = map.get(model.id) {
            // Already running, or cancelled but not stopped yet (a stalled
            // read can take a while to notice): carry on with that one.
            cancel.store(false, Ordering::Relaxed);
            return Ok(());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        map.insert(model.id, cancel.clone());
        cancel
    };
    std::thread::spawn(move || {
        let mut entry = Entry {
            app: app.clone(),
            id: model.id,
            removed: false,
        };
        let result = loop {
            let result = fetch(&app, &dir, model, &cancel);
            let downloads = app.state::<Downloads>();
            let mut map = downloads.lock();
            // Download was clicked again while this one was stopping.
            if result == Err(DownloadError::Cancelled) && !cancel.load(Ordering::Relaxed) {
                drop(map);
                continue;
            }
            map.remove(model.id);
            entry.removed = true;
            break result;
        };

        let (error, cancelled) = match result {
            Ok(_) => (None, false),
            Err(DownloadError::Cancelled) => (None, true),
            Err(DownloadError::Failed(error)) => (Some(error), false),
        };
        // The first model downloaded becomes the active one.
        if error.is_none() && !cancelled && app.state::<Store>().get().model.is_none() {
            let _ = select(&app, model);
        }
        let _ = app.emit(
            "model-downloaded",
            DownloadEvent {
                id: model.id,
                error,
                cancelled,
            },
        );
    });
    Ok(())
}

/// Downloads `model`, then the VAD model. The VAD model is best-effort: the
/// engine works without it (it only skips silence), so its failure is logged
/// and never blocks the speech model. It is tried again with the next download.
fn fetch(
    app: &AppHandle,
    dir: &Path,
    model: &'static ModelInfo,
    cancel: &AtomicBool,
) -> Result<PathBuf, DownloadError> {
    let path = models::download(dir, model, cancel, |done, total| {
        let _ = app.emit(
            "model-progress",
            Progress {
                id: model.id,
                done,
                total,
            },
        );
    })?;
    // Two model downloads can finish together; only one fetches the VAD
    // model, so they never write the same partial file. The other skips it.
    static VAD_FETCH: Mutex<()> = Mutex::new(());
    let fetching = match VAD_FETCH.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    };
    if fetching.is_some() {
        match models::download(dir, &models::VAD, cancel, |_, _| {}) {
            Ok(_) | Err(DownloadError::Cancelled) => {}
            Err(DownloadError::Failed(error)) => {
                log!("couldn't download the VAD model, continuing without it: {error}");
            }
        }
    }
    Ok(path)
}

/// Stops a running download soon. Its `model-downloaded` event then has
/// `cancelled: true`. The partial file is kept, so Download resumes it.
#[tauri::command]
pub fn cancel_download(downloads: State<'_, Downloads>, id: String) {
    if let Some(cancel) = downloads.lock().get(id.as_str()) {
        cancel.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn select_model(app: AppHandle, id: String) -> Result<(), String> {
    select(&app, find(&id)?)
}

/// Saves `model` as the active model and loads it in the background.
fn select(app: &AppHandle, model: &'static ModelInfo) -> Result<(), String> {
    let _select = SELECT.lock().unwrap_or_else(|e| e.into_inner());
    app.state::<Store>()
        .update(app, |s| s.model = Some(model.id.to_owned()))?;
    let ticket = app.state::<Engine>().begin_load(model.id);
    load_in_background(app.clone(), model, ticket);
    Ok(())
}

/// Loads the saved model at startup, if it has been downloaded.
pub fn load_selected(app: &AppHandle) {
    let Some(model) = app
        .state::<Store>()
        .get()
        .model
        .and_then(|id| models::find(&id))
    else {
        return;
    };
    let ticket = app.state::<Engine>().begin_load(model.id);
    load_in_background(app.clone(), model, ticket);
}

/// Loads `model` for `ticket`. Only the latest request changes the model;
/// an older one that finishes later is dropped quietly.
fn load_in_background(app: AppHandle, model: &'static ModelInfo, ticket: Ticket) {
    std::thread::spawn(move || {
        let engine = app.state::<Engine>();
        let files = models_dir(&app).and_then(|dir| {
            if !models::is_downloaded(&dir, model) {
                return Err(format!(
                    "The {} model isn't downloaded yet. Download it in Vocelle settings.",
                    model.name
                ));
            }
            let vad =
                models::is_downloaded(&dir, &models::VAD).then(|| models::path(&dir, &models::VAD));
            Ok((models::path(&dir, model), vad))
        });
        let result = match files {
            Ok((path, vad)) => engine.load(ticket, model, &path, vad.as_deref()),
            Err(error) => engine.fail_load(ticket, error),
        };
        let error = result.err();
        if let Some(error) = &error {
            log!("couldn't load {}: {error}", model.id);
            problem::report(&app, error);
        }
        let _ = app.emit(
            "model-loaded",
            ModelEvent {
                id: model.id,
                error,
            },
        );
    });
}
