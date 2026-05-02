// src-tauri/src/lib.rs
// Library entry point for the Tauri app. Boots the Builder, registers the
// command surface from commands.rs, applies window/hotkey wiring from
// window.rs, and resolves user-data paths into managed app state so every
// command handler shares the same UserPaths.

pub mod commands;
pub mod liquipedia;
pub mod storage;
pub mod types;
pub mod utils;
pub mod window;

use crate::storage::UserPaths;
use std::path::PathBuf;
use tauri::{Manager, RunEvent};

fn resolve_seed_path(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(resolved) = app
        .path()
        .resolve("data/builds.json", tauri::path::BaseDirectory::Resource)
    {
        if resolved.exists() {
            return resolved;
        }
    }
    if let Ok(resolved) = app
        .path()
        .resolve("../data/builds.json", tauri::path::BaseDirectory::Resource)
    {
        if resolved.exists() {
            return resolved;
        }
    }
    PathBuf::from("data/builds.json")
}

fn resolve_user_data(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

fn should_open_devtools_on_launch() -> bool {
    cfg!(debug_assertions) || std::env::var("BW_DEVTOOLS").as_deref() == Ok("1")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let user_data =
                resolve_user_data(app.handle()).expect("could not resolve app data dir");
            let seed_path = resolve_seed_path(app.handle());
            let paths = UserPaths::new(user_data, seed_path);
            app.manage(paths.clone());

            let app_handle = app.handle().clone();
            let paths_for_async = paths.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = storage::ensure_user_files(&paths_for_async).await {
                    log::error!("ensure_user_files failed: {err}");
                    return;
                }
                let settings = match storage::read_settings(&paths_for_async).await {
                    Ok(s) => s,
                    Err(err) => {
                        log::error!("read_settings failed on launch: {err}");
                        return;
                    }
                };
                if let Some(window) = crate::window::main_window(&app_handle) {
                    crate::window::set_opacity(&window, settings.overlay_opacity);
                    let _ = window.set_always_on_top(true);
                    let _ = window.show();
                    let _ = window.set_focus();
                    if should_open_devtools_on_launch() {
                        window.open_devtools();
                    }
                }
            });

            if let Some(window) = crate::window::main_window(app.handle()) {
                crate::window::install_always_on_top_keeper(&window);
            }

            if let Err(err) = crate::window::register_shortcuts(app.handle()) {
                log::error!("register_shortcuts failed: {err}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::builds_get,
            commands::builds_save,
            commands::settings_get,
            commands::settings_save,
            commands::liquipedia_preview_page,
            commands::liquipedia_import_page,
            commands::liquipedia_bulk_import,
            commands::liquipedia_check_updates,
            commands::liquipedia_refresh_build,
            commands::liquipedia_refresh_builds,
            commands::data_backup,
            commands::data_user_paths,
            commands::data_open_folder,
            commands::window_close,
            commands::window_toggle,
            commands::window_set_opacity,
            commands::window_toggle_devtools,
            commands::external_open,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                crate::window::unregister_shortcuts(app);
            }
        });
}
