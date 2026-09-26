mod elevation;
mod files;
mod fsutil;
mod junk;
mod startup;

use serde::Serialize;
use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Emitter};

/// Les analyses parcourent le disque : elles tournent hors du fil principal pour que la fenêtre reste fluide.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    spawn_blocking(f).await.map_err(|e| e.to_string())
}

/// Envoie à l'interface ce qui est en train d'être analysé, pour l'afficher sous l'anneau.
fn reporter(app: AppHandle) -> impl Fn(&str) {
    move |label: &str| {
        let _ = app.emit("progress", label);
    }
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    elevated: bool,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo { version: app.package_info().version.to_string(), elevated: elevation::is_elevated() }
}

#[tauri::command]
fn relaunch_as_admin(app: AppHandle) -> Result<(), String> {
    elevation::relaunch_as_admin()?;
    app.exit(0);
    Ok(())
}

#[derive(Serialize)]
struct DiskInfo {
    name: String,
    total: u64,
    free: u64,
}

/// Le disque où Windows est installé (C: en général).
#[tauri::command]
fn disk_info() -> Option<DiskInfo> {
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "/".into());
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .find(|d| d.mount_point().to_string_lossy().trim_end_matches('\\').eq_ignore_ascii_case(&system))
        .or_else(|| disks.list().iter().max_by_key(|d| d.total_space()))
        .map(|d| DiskInfo {
            name: system.trim_end_matches('\\').to_string(),
            total: d.total_space(),
            free: d.available_space(),
        })
}

#[tauri::command]
async fn scan_junk(app: AppHandle) -> Result<Vec<junk::JunkItem>, String> {
    blocking(move || junk::scan(&reporter(app))).await
}

#[tauri::command]
async fn clean_junk(app: AppHandle, ids: Vec<String>) -> Result<junk::CleanReport, String> {
    blocking(move || junk::clean(&ids, &reporter(app))).await
}

#[tauri::command]
async fn find_large_files(app: AppHandle, min_mb: u64) -> Result<Vec<files::FileEntry>, String> {
    blocking(move || files::large_files(&files::user_roots(), min_mb * 1024 * 1024, 60, &reporter(app))).await
}

#[tauri::command]
async fn find_duplicates(app: AppHandle) -> Result<Vec<files::DuplicateGroup>, String> {
    blocking(move || files::duplicates(&files::user_roots(), 100 * 1024, &reporter(app))).await
}

#[tauri::command]
fn reveal_file(path: String) -> Result<(), String> {
    files::reveal(&files::user_roots(), &path)
}

#[tauri::command]
async fn move_to_trash(paths: Vec<String>) -> Result<files::TrashReport, String> {
    blocking(move || files::move_to_trash(&files::user_roots(), &paths)).await
}

#[tauri::command]
fn list_startup_apps() -> Vec<startup::StartupApp> {
    startup::list()
}

#[tauri::command]
fn set_startup_app(id: String, enabled: bool) -> Result<(), String> {
    startup::set(&id, enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            relaunch_as_admin,
            disk_info,
            scan_junk,
            clean_junk,
            find_large_files,
            find_duplicates,
            move_to_trash,
            reveal_file,
            list_startup_apps,
            set_startup_app,
        ])
        .run(tauri::generate_context!())
        .expect("impossible de lancer Kaury Clean");
}
