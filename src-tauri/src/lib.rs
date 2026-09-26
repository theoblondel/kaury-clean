mod elevation;
mod files;
mod fsutil;
mod junk;
mod maintenance;
mod memory;
mod organize;
mod startup;
mod uninstall;
mod update;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

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

/// Lance une recherche qu'on peut arrêter avec `cancel_search`.
async fn search<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    files::CANCEL.store(false, Ordering::Relaxed);
    let result = blocking(f).await?;
    if files::cancelled() {
        return Err("Recherche arrêtée".into());
    }
    Ok(result)
}

#[tauri::command]
fn cancel_search() {
    files::CANCEL.store(true, Ordering::Relaxed);
}

#[tauri::command]
async fn find_large_files(app: AppHandle, min_mb: u64) -> Result<Vec<files::FileEntry>, String> {
    search(move || files::large_files(&files::user_roots(), min_mb * 1024 * 1024, 60, &reporter(app))).await
}

#[tauri::command]
async fn find_duplicates(app: AppHandle) -> Result<Vec<files::DuplicateGroup>, String> {
    search(move || files::duplicates(&files::user_roots(), 100 * 1024, &reporter(app))).await
}

#[tauri::command]
async fn find_old_downloads(app: AppHandle, min_days: u64) -> Result<Vec<files::FileEntry>, String> {
    let downloads = dirs::download_dir().ok_or("Dossier Téléchargements introuvable")?;
    search(move || files::old_downloads(&downloads, min_days, &reporter(app))).await
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

// ---------- Ranger mes fichiers ----------

/// Seuls le Bureau et les Téléchargements peuvent être rangés.
fn organize_dir(folder: &str) -> Result<PathBuf, String> {
    match folder {
        "downloads" => dirs::download_dir(),
        "desktop" => dirs::desktop_dir(),
        _ => None,
    }
    .ok_or_else(|| "Dossier introuvable".into())
}

/// Rangé dans le dossier de données de l'appli : le désinstalleur le supprime avec le reste.
fn undo_log() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("studio.kaury.clean")
        .join("dernier-rangement.json")
}

#[tauri::command]
fn organize_plan(folder: String) -> Result<Vec<organize::PlanGroup>, String> {
    Ok(organize::plan(&organize_dir(&folder)?))
}

#[tauri::command]
async fn organize_apply(folder: String) -> Result<organize::OrganizeReport, String> {
    let dir = organize_dir(&folder)?;
    blocking(move || organize::apply(&dir, &undo_log())).await
}

#[tauri::command]
fn organize_can_undo() -> bool {
    organize::can_undo(&undo_log())
}

#[tauri::command]
async fn organize_undo() -> Result<organize::OrganizeReport, String> {
    blocking(|| organize::undo(&undo_log())).await
}

// ---------- Applis installées ----------

#[tauri::command]
async fn list_installed_apps() -> Result<Vec<uninstall::InstalledApp>, String> {
    blocking(uninstall::list).await
}

#[tauri::command]
fn uninstall_app(id: String) -> Result<(), String> {
    uninstall::uninstall(&id)
}

// ---------- Mémoire ----------

#[tauri::command]
async fn memory_status() -> Result<memory::MemoryStatus, String> {
    blocking(memory::status).await
}

#[tauri::command]
async fn close_app(exe: String, force: bool) -> Result<bool, String> {
    blocking(move || memory::close(&exe, force)).await?
}

// ---------- Maintenance ----------

#[tauri::command]
fn list_maintenance() -> Vec<maintenance::TaskInfo> {
    maintenance::list()
}

#[tauri::command]
async fn run_maintenance(app: AppHandle, id: String) -> Result<maintenance::TaskResult, String> {
    blocking(move || maintenance::run(&id, &reporter(app))).await?
}

// ---------- Liens et mises à jour ----------

/// Les seules adresses que l'appli sait ouvrir.
#[tauri::command]
fn open_link(link: String) -> Result<(), String> {
    let url = match link.as_str() {
        "site" => "https://kaury.studio",
        "behance" => "https://www.behance.net/kaurystudio",
        "instagram" => "https://www.instagram.com/kaury.studio/",
        "email" => "mailto:hello@kaury.studio",
        "releases" => "https://github.com/theoblondel/kaury-clean/releases",
        _ => return Err("Lien inconnu".into()),
    };
    elevation::shell_open(url, "")
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<update::UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    blocking(move || update::check(&current)).await?
}

/// Télécharge la nouvelle version, lance son installeur puis ferme l'appli pour qu'il puisse la remplacer.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let current = app.package_info().version.to_string();
    let emitter = app.clone();
    let path = blocking(move || {
        update::download(&current, &|pct| {
            let _ = emitter.emit("progress", format!("Téléchargement · {pct} %"));
        })
    })
    .await??;
    update::launch_installer(&path)?;
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Taille et position de la fenêtre retrouvées à chaque ouverture.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            app_info,
            open_link,
            check_update,
            install_update,
            relaunch_as_admin,
            disk_info,
            scan_junk,
            clean_junk,
            find_large_files,
            find_duplicates,
            find_old_downloads,
            cancel_search,
            organize_plan,
            organize_apply,
            organize_can_undo,
            organize_undo,
            list_installed_apps,
            uninstall_app,
            memory_status,
            close_app,
            list_maintenance,
            run_maintenance,
            move_to_trash,
            reveal_file,
            list_startup_apps,
            set_startup_app,
        ])
        .run(tauri::generate_context!())
        .expect("impossible de lancer Kaury Clean");
}
