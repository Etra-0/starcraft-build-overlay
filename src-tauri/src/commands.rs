// src-tauri/src/commands.rs
// Tauri command surface that exposes the OverlayAPI contract from
// src/shared/types.ts to the frontend. Each #[tauri::command] wraps
// storage / liquipedia / window helpers and returns plain
// serde-serialisable values to the renderer. Long-running imports emit
// "liquipedia:progress" events; the global hotkeys live in window.rs
// and emit "hotkey" events directly.

use crate::liquipedia::import::{self, ProgressFn};
use crate::liquipedia::updates;
use crate::storage::{self, UserPaths};
use crate::types::{
    Build, BuildsData, BulkImportOptions, CheckUpdatesResult, ImportOptions,
    ImportSinglePageResult, RefreshBuildsOptions, RefreshBuildsResult, Settings, UserDataPaths,
};
use crate::{liquipedia, window as winmod};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

fn err_string<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn make_progress(app: &AppHandle) -> ProgressFn {
    let app = app.clone();
    Arc::new(move |message: &str| {
        let _ = app.emit("liquipedia:progress", message.to_string());
    })
}

#[tauri::command]
pub async fn builds_get(paths: State<'_, UserPaths>) -> Result<BuildsData, String> {
    storage::read_builds(paths.inner())
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn builds_save(
    paths: State<'_, UserPaths>,
    builds: BuildsData,
) -> Result<BuildsData, String> {
    storage::save_builds(paths.inner(), builds)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn settings_get(paths: State<'_, UserPaths>) -> Result<Settings, String> {
    storage::read_settings(paths.inner())
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn settings_save(
    paths: State<'_, UserPaths>,
    settings: Value,
) -> Result<Settings, String> {
    storage::save_settings(paths.inner(), settings)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_preview_page(
    app: AppHandle,
    paths: State<'_, UserPaths>,
    input: String,
) -> Result<Vec<Build>, String> {
    let progress = make_progress(&app);
    import::preview_single_page(paths.inner(), &input, Some(&progress))
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_import_page(
    app: AppHandle,
    paths: State<'_, UserPaths>,
    input: String,
    options: ImportOptions,
) -> Result<ImportSinglePageResult, String> {
    let progress = make_progress(&app);
    import::import_single_page(
        paths.inner(),
        &input,
        options.update_existing,
        Some(&progress),
    )
    .await
    .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_bulk_import(
    app: AppHandle,
    paths: State<'_, UserPaths>,
    options: BulkImportOptions,
) -> Result<crate::types::BulkImportResult, String> {
    let progress = make_progress(&app);
    import::bulk_import(paths.inner(), options, Some(&progress))
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_check_updates(
    app: AppHandle,
    paths: State<'_, UserPaths>,
) -> Result<CheckUpdatesResult, String> {
    let progress = make_progress(&app);
    updates::check_for_updates(paths.inner(), Some(&progress))
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_refresh_build(
    app: AppHandle,
    paths: State<'_, UserPaths>,
    build_id: String,
) -> Result<Build, String> {
    let progress = make_progress(&app);
    updates::refresh_build(paths.inner(), &build_id, Some(&progress))
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn liquipedia_refresh_builds(
    app: AppHandle,
    paths: State<'_, UserPaths>,
    build_ids: Vec<String>,
    options: RefreshBuildsOptions,
) -> Result<RefreshBuildsResult, String> {
    let progress = make_progress(&app);
    updates::refresh_builds(paths.inner(), &build_ids, options, Some(&progress))
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn data_backup(paths: State<'_, UserPaths>) -> Result<String, String> {
    storage::backup_builds(paths.inner())
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn data_user_paths(paths: State<'_, UserPaths>) -> Result<UserDataPaths, String> {
    Ok(paths.inner().to_dto())
}

#[tauri::command]
pub fn data_open_folder(paths: State<'_, UserPaths>) -> Result<(), String> {
    let dir = paths.inner().user_data.clone();
    open_native_path(&dir).map_err(err_string)
}

fn open_native_path(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(path)
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_close(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn window_toggle(window: WebviewWindow) {
    winmod::toggle_visible(&window);
}

#[tauri::command]
pub fn window_set_opacity(window: WebviewWindow, value: f64) {
    winmod::set_opacity(&window, value);
}

#[tauri::command]
pub fn window_toggle_devtools(window: WebviewWindow) {
    winmod::toggle_devtools(&window);
}

#[tauri::command]
pub fn external_open(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("only http(s) URLs are allowed".to_string());
    }
    app.opener().open_url(url, None::<&str>).map_err(err_string)
}

/// Suppress an unused-import warning when the `liquipedia` module is brought
/// in only for its public command wrappers above (rust-analyzer occasionally
/// flags this otherwise). The use is real - all import/update commands route
/// through the `crate::liquipedia` modules.
#[allow(dead_code)]
fn _module_anchor() {
    let _ = liquipedia::parser::strip_wiki_markup("");
}
