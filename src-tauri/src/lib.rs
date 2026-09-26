mod elevation;
mod files;
mod fsutil;
mod garde;
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

/// Tes fichiers perso se déplacent (corbeille, rangement) seulement sans les droits administrateur :
/// ils n'en ont jamais besoin, et en administrateur un programme pourrait remplacer un dossier par un
/// raccourci au bon moment pour faire déplacer un fichier dans Windows ou hors de Windows.
fn without_admin() -> Result<(), String> {
    if elevation::is_elevated() {
        return Err(concat!(
            "par sécurité, tes fichiers perso ne se déplacent pas en mode administrateur. ",
            "Ferme Kaury Clean et rouvre-le normalement."
        )
        .into());
    }
    Ok(())
}

#[tauri::command]
async fn move_to_trash(paths: Vec<String>) -> Result<files::TrashReport, String> {
    without_admin()?;
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
    let dir = match folder {
        "downloads" => dirs::download_dir(),
        "desktop" => dirs::desktop_dir(),
        _ => None,
    }
    .ok_or("Dossier introuvable")?;
    if !garde::user_dir_allowed(&dir) {
        return Err("Ce dossier n'est pas dans ton dossier personnel : Kaury Clean n'y touche pas en administrateur".into());
    }
    Ok(dir)
}

/// Les deux dossiers rangeables : l'annulation ne déplace rien ailleurs.
fn organize_dirs() -> Vec<PathBuf> {
    ["downloads", "desktop"].iter().filter_map(|f| organize_dir(f).ok()).collect()
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
    without_admin()?;
    let dir = organize_dir(&folder)?;
    blocking(move || organize::apply(&dir, &undo_log())).await
}

#[tauri::command]
fn organize_can_undo() -> bool {
    organize::can_undo(&undo_log())
}

#[tauri::command]
async fn organize_undo() -> Result<organize::OrganizeReport, String> {
    without_admin()?;
    blocking(|| organize::undo(&undo_log(), &organize_dirs())).await
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

/// Télécharge la nouvelle version, vérifie sa signature, lance son installeur puis ferme l'appli pour
/// qu'il puisse la remplacer.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let current = app.package_info().version.to_string();
    let emitter = app.clone();
    let (locked, path) = blocking(move || {
        update::download(&current, &|pct| {
            let _ = emitter.emit("progress", format!("Téléchargement · {pct} %"));
        })
    })
    .await??;
    // L'installeur vérifié reste verrouillé jusqu'à son lancement : personne ne peut l'échanger entre-temps.
    update::launch_installer(&path)?;
    drop(locked);
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Les variables WEBVIEW2_* (que n'importe quel programme de ton compte peut définir) permettent de
    // remplacer le moteur d'affichage par un autre programme, ou d'ouvrir un accès de débogage à la
    // fenêtre. En administrateur, ce programme hériterait de nos droits : on les retire avant tout.
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().to_ascii_uppercase().starts_with("WEBVIEW2_") {
            std::env::remove_var(name);
        }
    }

    let mut builder = tauri::Builder::default();
    // Taille et position de la fenêtre retrouvées à chaque ouverture. Pas en administrateur : le
    // fichier est dans un dossier de ton compte, et l'écrire avec ces droits pourrait être détourné
    // pour écraser un fichier de Windows.
    if !elevation::is_elevated() {
        builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    }
    builder
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
