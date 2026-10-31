//! Settings-window commands for speech models, and loading the chosen model.

use std::{collections::HashSet, path::PathBuf, sync::Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    asr::Engine,
    models::{self, ModelInfo},
    settings,
};

/// Model ids currently downloading, so a double click doesn't start two downloads.
#[derive(Default)]
pub struct Downloads(Mutex<HashSet<&'static str>>);

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

#[tauri::command]
pub fn list_models(
    app: AppHandle,
    engine: State<'_, Engine>,
    downloads: State<'_, Downloads>,
) -> Result<Vec<ModelStatus>, String> {
    let dir = models_dir(&app)?;
    let selected = settings::load(&app).model;
    let loaded = engine.loaded_model();
    let downloading = downloads.0.lock().unwrap();
    let recommended = models::recommended().id;
    Ok(models::CATALOG
        .iter()
        .map(|m| ModelStatus {
            info: *m,
            downloaded: models::is_downloaded(&dir, m),
            downloading: downloading.contains(m.id),
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
    if !downloads.0.lock().unwrap().insert(model.id) {
        return Ok(());
    }
    std::thread::spawn(move || {
        let result = models::download(&dir, &models::VAD, |_, _| {}).and_then(|_| {
            models::download(&dir, model, |done, total| {
                let _ = app.emit(
                    "model-progress",
                    Progress {
                        id: model.id,
                        done,
                        total,
                    },
                );
            })
        });
        app.state::<Downloads>().0.lock().unwrap().remove(model.id);

        let error = result.err();
        // The first model downloaded becomes the active one.
        if error.is_none() && settings::load(&app).model.is_none() {
            let _ = select(&app, model);
        }
        let _ = app.emit(
            "model-downloaded",
            ModelEvent {
                id: model.id,
                error,
            },
        );
    });
    Ok(())
}

#[tauri::command]
pub fn select_model(app: AppHandle, id: String) -> Result<(), String> {
    select(&app, find(&id)?)
}

/// Saves `model` as the active model and loads it in the background.
fn select(app: &AppHandle, model: &'static ModelInfo) -> Result<(), String> {
    let mut s = settings::load(app);
    s.model = Some(model.id.to_owned());
    settings::save(app, &s)?;
    load_in_background(app.clone(), model);
    Ok(())
}

/// Loads the saved model at startup, if it has been downloaded.
pub fn load_selected(app: &AppHandle) {
    let Some(model) = settings::load(app).model.and_then(|id| models::find(&id)) else {
        return;
    };
    load_in_background(app.clone(), model);
}

fn load_in_background(app: AppHandle, model: &'static ModelInfo) {
    std::thread::spawn(move || {
        let error = models_dir(&app).and_then(|dir| {
            if !models::is_downloaded(&dir, model) {
                return Err(format!("{} isn't downloaded yet", model.name));
            }
            let vad = models::path(&dir, &models::VAD);
            let vad = models::is_downloaded(&dir, &models::VAD).then_some(vad.as_path());
            app.state::<Engine>()
                .load(model, &models::path(&dir, model), vad)
        });
        let _ = app.emit(
            "model-loaded",
            ModelEvent {
                id: model.id,
                error: error.err(),
            },
        );
    });
}
