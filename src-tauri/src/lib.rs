mod files;
mod fsutil;
mod junk;
mod startup;

use serde::Serialize;
use tauri::async_runtime::spawn_blocking;

/// Les analyses parcourent le disque : elles tournent hors du fil principal pour que la fenêtre reste fluide.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    spawn_blocking(f).await.map_err(|e| e.to_string())
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
async fn scan_junk() -> Result<Vec<junk::JunkItem>, String> {
    blocking(junk::scan).await
}

#[tauri::command]
async fn clean_junk(ids: Vec<String>) -> Result<junk::CleanReport, String> {
    blocking(move || junk::clean(&ids)).await
}

#[tauri::command]
async fn find_large_files(min_mb: u64) -> Result<Vec<files::FileEntry>, String> {
    blocking(move || files::large_files(&files::user_roots(), min_mb * 1024 * 1024, 60)).await
}

#[tauri::command]
async fn find_duplicates() -> Result<Vec<files::DuplicateGroup>, String> {
    blocking(|| files::duplicates(&files::user_roots(), 100 * 1024)).await
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
            disk_info,
            scan_junk,
            clean_junk,
            find_large_files,
            find_duplicates,
            move_to_trash,
            list_startup_apps,
            set_startup_app,
        ])
        .run(tauri::generate_context!())
        .expect("impossible de lancer Kaury Clean");
}
