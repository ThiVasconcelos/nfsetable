//! Tauri commands: thin wrappers over `nfsetable-core`. Heavy work runs on the blocking thread pool so
//! the UI never freezes; long batches report progress through events.

use crate::results::{self, ResultCache};
use crate::{data_dir, profiles, store};
use nfsetable_core::{
    builtin_fields, builtin_profiles, AppInfo, DocResult, Engine, ExportRequest, FieldDef, Profile,
    Progress, Rect, RegionRead, RenderedPage, Rule, RuleTest, ScanOptions, ScanResult, TaxCatalog,
    TaxInput, TaxReport, NET_VALUE_FIELD,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use tauri::{AppHandle, Emitter, State};

pub const EXTRACT_PROGRESS: &str = "extract-progress";
pub const TEST_PROGRESS: &str = "test-progress";

/// Shared state of the app.
pub struct AppState {
    /// The engine, or the pt-BR reason why PDFium could not be loaded.
    engine: Result<Arc<Engine>, String>,
    /// The app's own data folder (also where the choice of data folder is kept).
    default_dir: PathBuf,
    /// Folder with the profiles and stored documents; can be changed at runtime.
    data_dir: RwLock<PathBuf>,
    data_dir_error: RwLock<Option<String>>,
    /// Extraction results of earlier runs, kept in the data folder in use.
    results: Arc<Mutex<ResultCache>>,
    /// Held while the results are written, so writes land one at a time and in order.
    results_writer: Arc<Mutex<()>>,
}

impl AppState {
    pub fn new(engine: Result<Engine, String>, default_dir: PathBuf) -> Self {
        let (data_dir, data_dir_error) = data_dir::resolve(&default_dir);
        AppState {
            engine: engine.map(Arc::new),
            results: Arc::new(Mutex::new(ResultCache::load(&data_dir))),
            results_writer: Arc::new(Mutex::new(())),
            default_dir,
            data_dir: RwLock::new(data_dir),
            data_dir_error: RwLock::new(data_dir_error),
        }
    }

    fn engine(&self) -> Result<Arc<Engine>, String> {
        self.engine.clone()
    }

    fn data_dir(&self) -> PathBuf {
        read_lock(&self.data_dir)
    }

    fn results(&self) -> MutexGuard<'_, ResultCache> {
        lock(&self.results)
    }

    /// Writes the kept results if they changed, off the async runtime.
    async fn save_results(&self) -> Result<(), String> {
        let (results, writer) = (self.results.clone(), self.results_writer.clone());
        blocking(move || {
            let _turn = lock(&writer);
            let Some((path, json)) = lock(&results).pending_save() else {
                return Ok(());
            };
            results::write(&path, &json).map_err(|e| {
                lock(&results).save_failed();
                format!("Não foi possível guardar as leituras: {e}")
            })
        })
        .await?
    }

    fn profiles_dir(&self) -> PathBuf {
        self.data_dir().join("profiles")
    }

    fn store_dir(&self) -> PathBuf {
        self.data_dir().join("store")
    }

    /// Built-in profiles first, then the user's.
    fn all_profiles(&self) -> Vec<Profile> {
        let mut all = builtin_profiles();
        all.extend(profiles::load_all(&self.profiles_dir()));
        all
    }

    fn info(&self, app: &AppHandle) -> AppInfo {
        AppInfo {
            version: app.package_info().version.to_string(),
            pdfium_ok: self.engine.is_ok(),
            pdfium_error: self.engine.as_ref().err().cloned(),
            profiles_dir: self.profiles_dir().to_string_lossy().into_owned(),
            data_dir: self.data_dir().to_string_lossy().into_owned(),
            default_data_dir: self.default_dir.to_string_lossy().into_owned(),
            data_dir_error: read_lock(&self.data_dir_error),
        }
    }
}

fn net_value_field() -> FieldDef {
    builtin_fields()
        .into_iter()
        .find(|f| f.id == NET_VALUE_FIELD)
        .expect("net_value is a built-in field")
}

/// The value behind `lock`; a lock poisoned by a panic still holds a usable value.
fn read_lock<T: Clone>(lock: &RwLock<T>) -> T {
    lock.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// The value behind a mutex (poisoned or not).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Replaces the value behind `lock` (poisoned or not).
fn set_lock<T>(lock: &RwLock<T>, value: T) {
    *lock.write().unwrap_or_else(PoisonError::into_inner) = value;
}

/// Runs `work` on every path, in order, emitting `event` with the progress after each one.
fn map_with_progress<T>(
    app: &AppHandle,
    event: &str,
    paths: &[String],
    mut work: impl FnMut(&Path) -> T,
) -> Vec<T> {
    let total = u32::try_from(paths.len()).unwrap_or(u32::MAX);
    paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let result = work(Path::new(path));
            let done = u32::try_from(i + 1).unwrap_or(u32::MAX);
            let _ = app.emit(event, Progress { done, total });
            result
        })
        .collect()
}

/// Runs blocking work off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("Erro interno: {e}"))
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    state.info(&app)
}

/// Changes the folder of the persistent data (`path` = None goes back to the default). With
/// `copy`, the current profiles and stored documents are copied there first.
#[tauri::command]
pub fn set_data_dir(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
    copy: bool,
) -> Result<AppInfo, String> {
    let target = path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    let new_dir = data_dir::switch(
        &state.default_dir,
        &state.data_dir(),
        target.as_deref(),
        copy,
    )?;
    // The results kept so far stay with the old folder; the new folder has its own.
    {
        let _turn = lock(&state.results_writer);
        let pending = state.results().pending_save();
        if let Some((old_path, json)) = pending {
            let _ = results::write(&old_path, &json);
        }
    }
    *state.results() = ResultCache::load(&new_dir);
    set_lock(&state.data_dir, new_dir);
    set_lock(&state.data_dir_error, None);
    Ok(state.info(&app))
}

#[tauri::command]
pub async fn scan_sources(options: ScanOptions) -> Result<ScanResult, String> {
    blocking(move || nfsetable_core::scan(&options)).await
}

/// Results read in earlier runs for these cache keys (see `extract_documents`), valid for the
/// current profiles.
#[tauri::command]
pub async fn cached_results(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<HashMap<String, DocResult>, String> {
    let key = results::key(&state.all_profiles());
    Ok(state.results().get(&key, &keys))
}

/// Extracts `paths` and keeps the results for the next runs under `keys` (same order: the UI's
/// cache key of each file, empty for files the scan could not read).
#[tauri::command]
pub async fn extract_documents(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    keys: Vec<String>,
) -> Result<Vec<DocResult>, String> {
    if keys.len() != paths.len() {
        return Err("Erro interno: uma chave por arquivo.".to_string());
    }
    let engine = state.engine()?;
    let profiles = state.all_profiles();
    let key = results::key(&profiles);
    let fields = builtin_fields();
    let docs = blocking(move || {
        map_with_progress(&app, EXTRACT_PROGRESS, &paths, |path| {
            engine.extract(path, &profiles, &fields)
        })
    })
    .await?;
    let due = {
        let mut cache = state.results();
        for (doc, id) in docs.iter().zip(&keys) {
            cache.insert(&key, id, doc);
        }
        cache.save_due()
    };
    if due {
        // A long reading keeps what it read even if the app closes before the end.
        let _ = state.save_results().await;
    }
    Ok(docs)
}

/// Writes the kept results to disk (the UI calls it when a reading ends).
#[tauri::command]
pub async fn save_cached_results(state: State<'_, AppState>) -> Result<(), String> {
    state.save_results().await
}

#[tauri::command]
pub async fn render_page(
    state: State<'_, AppState>,
    path: String,
    page: u32,
    width: u32,
) -> Result<RenderedPage, String> {
    let engine = state.engine()?;
    blocking(move || engine.render_page(Path::new(&path), page, width))
        .await?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_region(
    state: State<'_, AppState>,
    path: String,
    page: u32,
    rect: Rect,
) -> Result<RegionRead, String> {
    let engine = state.engine()?;
    let kind = net_value_field().kind;
    blocking(move || engine.read_region(Path::new(&path), page, rect, kind))
        .await?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_rule(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    rule: Rule,
) -> Result<Vec<RuleTest>, String> {
    let engine = state.engine()?;
    nfsetable_core::profile::validate_rule(&rule).map_err(|e| e.to_string())?;
    let field = net_value_field();
    blocking(move || {
        map_with_progress(&app, TEST_PROGRESS, &paths, |path| {
            engine.test_rule(path, &field, &rule)
        })
    })
    .await
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Vec<Profile> {
    state.all_profiles()
}

#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, profile: Profile) -> Result<Profile, String> {
    if builtin_profiles().iter().any(|p| p.id == profile.id) {
        return Err("Perfis embutidos não podem ser alterados.".to_string());
    }
    profiles::save(&state.profiles_dir(), profile)
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if builtin_profiles().iter().any(|p| p.id == id) {
        return Err("Perfis embutidos não podem ser excluídos.".to_string());
    }
    profiles::delete(&state.profiles_dir(), &id)
}

#[tauri::command]
pub async fn export_table(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<(), String> {
    // Only the PDF report needs PDFium; the other formats work without it.
    let engine = state.engine().ok();
    blocking(move || nfsetable_core::export_table(&request, engine.as_deref()))
        .await?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tax_report(input: TaxInput) -> Result<TaxReport, String> {
    blocking(move || nfsetable_core::tax_report(&input))
        .await?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tax_catalog() -> TaxCatalog {
    nfsetable_core::tax_catalog()
}

#[tauri::command]
pub fn read_store(
    state: State<'_, AppState>,
    name: String,
) -> Result<Option<serde_json::Value>, String> {
    store::read(&state.store_dir(), &name)
}

#[tauri::command]
pub fn write_store(
    state: State<'_, AppState>,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    store::write(&state.store_dir(), &name, &value)
}

#[tauri::command]
pub fn delete_store(state: State<'_, AppState>, name: String) -> Result<(), String> {
    store::delete(&state.store_dir(), &name)
}
