// Hides the console window of release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod data_dir;
mod files;
mod profiles;
mod store;

use commands::AppState;
use nfsetable_core::Engine;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let engine = Engine::new(&pdfium_search_dirs(handle)).map_err(|e| e.to_string());
            // NFSETABLE_DATA_DIR forces the data folder (portable use, tests); otherwise the
            // app's own data folder.
            let default_dir = std::env::var_os("NFSETABLE_DATA_DIR")
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    handle
                        .path()
                        .app_data_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                });
            app.manage(AppState::new(engine, default_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::set_data_dir,
            commands::scan_sources,
            commands::extract_documents,
            commands::render_page,
            commands::read_region,
            commands::test_rule,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::export_table,
            commands::tax_report,
            commands::tax_catalog,
            commands::read_store,
            commands::write_store,
            commands::delete_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}

/// Where the bundled PDFium library may be: the resource folder (Windows installs it next to the
/// executable, Linux and macOS under `pdfium/`), plus the repository's `vendor/` folder in debug
/// builds. `Engine::new` also tries the executable's folder and the system libraries.
fn pdfium_search_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(resources) = app.path().resource_dir() {
        dirs.push(resources.join("pdfium"));
        dirs.push(resources);
    }
    if cfg!(debug_assertions) {
        dirs.push(nfsetable_core::dev_pdfium_dir());
    }
    dirs
}
