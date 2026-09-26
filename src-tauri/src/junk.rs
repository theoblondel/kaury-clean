//! Fichiers inutiles : caches et fichiers temporaires que Windows et les applis recréent tout seuls.
//!
//! La liste des dossiers est fixée ici, côté Rust. L'interface n'envoie que des identifiants,
//! jamais de chemins : impossible de lui faire vider un autre dossier.

use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::fsutil::{clean_dir, dir_size};
use crate::garde;

const DAY: Duration = Duration::from_secs(24 * 3600);

struct Target {
    id: &'static str,
    group: &'static str,
    name: &'static str,
    detail: &'static str,
    paths: Vec<PathBuf>,
    /// Les fichiers plus récents que ça sont gardés (un installeur en cours peut encore en avoir besoin).
    min_age: Duration,
    /// Processus qui verrouillent ces fichiers tant qu'ils tournent : (nom du .exe, nom affiché).
    processes: &'static [(&'static str, &'static str)],
    /// Dossier protégé par Windows : sans les droits administrateur, presque rien ne peut partir.
    needs_admin: bool,
}

#[derive(Serialize)]
pub struct JunkItem {
    id: String,
    group: String,
    name: String,
    detail: String,
    bytes: u64,
    files: u64,
    /// Nom de l'appli ouverte qui empêche un nettoyage complet, s'il y en a une.
    running: Option<String>,
    needs_admin: bool,
}

#[derive(Serialize, Default)]
pub struct CleanReport {
    freed: u64,
    removed: u64,
    skipped: u64,
}

fn env_path(var: &str) -> Option<PathBuf> {
    env::var_os(var).map(PathBuf::from)
}

fn join(base: &Option<PathBuf>, rel: &str) -> Vec<PathBuf> {
    base.iter().map(|b| b.join(rel)).collect()
}

/// Dossiers de profils d'un navigateur basé sur Chromium (Default, Profile 1, ...).
fn chromium_caches(user_data: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(user_data) = user_data else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&user_data) else { return vec![] };
    let mut out = vec![];
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "Default" || name.starts_with("Profile ") {
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                out.push(entry.path().join(sub));
            }
        }
    }
    out.push(user_data.join("ShaderCache"));
    out.push(user_data.join("GrShaderCache"));
    out
}

fn firefox_caches(local: &Option<PathBuf>) -> Vec<PathBuf> {
    let Some(local) = local else { return vec![] };
    let Ok(entries) = std::fs::read_dir(local.join("Mozilla/Firefox/Profiles")) else { return vec![] };
    entries.flatten().map(|e| e.path().join("cache2")).collect()
}

fn targets() -> Vec<Target> {
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    // Dossiers de Windows donnés par Windows, jamais par une variable qu'un programme pourrait détourner.
    let windir = Some(garde::windows_dir());
    let program_data = Some(program_data());

    let mut crash = join(&local, "CrashDumps");
    crash.extend(join(&local, "Microsoft/Windows/WER"));
    crash.extend(join(&program_data, "Microsoft/Windows/WER/ReportArchive"));
    crash.extend(join(&program_data, "Microsoft/Windows/WER/ReportQueue"));

    let mut shaders = join(&local, "D3DSCache");
    shaders.extend(join(&local, "NVIDIA/DXCache"));
    shaders.extend(join(&local, "NVIDIA/GLCache"));
    shaders.extend(join(&local, "AMD/DxCache"));

    let mut spotify = join(&local, "Spotify/Storage");
    spotify.extend(join(&local, "Spotify/Data"));

    let mut discord = vec![];
    for sub in ["Cache", "Code Cache", "GPUCache"] {
        discord.extend(join(&roaming, &format!("discord/{sub}")));
    }

    let mut adobe = join(&roaming, "Adobe/Common/Media Cache Files");
    adobe.extend(join(&roaming, "Adobe/Common/Peak Files"));

    vec![
        Target {
            id: "user_temp",
            group: "system",
            name: "Fichiers temporaires",
            detail: "Dossier Temp de ton compte",
            paths: vec![env::temp_dir()],
            min_age: DAY,
            processes: &[],
            needs_admin: false,
        },
        Target {
            id: "windows_temp",
            group: "system",
            name: "Fichiers temporaires Windows",
            detail: "C:\\Windows\\Temp",
            paths: join(&windir, "Temp"),
            min_age: DAY,
            processes: &[],
            needs_admin: true,
        },
        Target {
            id: "windows_update",
            group: "system",
            name: "Téléchargements Windows Update",
            detail: "Mises à jour déjà installées",
            paths: join(&windir, "SoftwareDistribution/Download"),
            min_age: DAY,
            processes: &[],
            needs_admin: true,
        },
        Target {
            id: "delivery_optimization",
            group: "system",
            name: "Optimisation de la distribution",
            detail: "Mises à jour Windows partagées avec d'autres PC",
            paths: join(&windir, "ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache"),
            min_age: Duration::ZERO,
            processes: &[],
            needs_admin: true,
        },
        Target {
            id: "crash_reports",
            group: "system",
            name: "Rapports d'erreur",
            detail: "Rapports de plantage et fichiers dump",
            paths: crash,
            min_age: Duration::ZERO,
            processes: &[],
            needs_admin: false,
        },
        Target {
            id: "shader_cache",
            group: "system",
            name: "Cache de la carte graphique",
            detail: "DirectX, NVIDIA et AMD",
            paths: shaders,
            min_age: Duration::ZERO,
            processes: &[],
            needs_admin: false,
        },
        Target {
            id: "adobe_media_cache",
            group: "apps",
            name: "Cache média Adobe",
            detail: "Premiere Pro et After Effects",
            paths: adobe,
            min_age: Duration::ZERO,
            processes: &[("Adobe Premiere Pro.exe", "Premiere Pro"), ("AfterFX.exe", "After Effects"), ("Adobe Media Encoder.exe", "Media Encoder")],
            needs_admin: false,
        },
        Target {
            id: "spotify",
            group: "apps",
            name: "Spotify",
            detail: "Musique mise en cache : elle se retélécharge quand tu l'écoutes",
            paths: spotify,
            min_age: Duration::ZERO,
            processes: &[("Spotify.exe", "Spotify")],
            needs_admin: false,
        },
        Target {
            id: "discord",
            group: "apps",
            name: "Discord",
            detail: "Images et vidéos déjà vues",
            paths: discord,
            min_age: Duration::ZERO,
            processes: &[("Discord.exe", "Discord")],
            needs_admin: false,
        },
        Target {
            id: "steam",
            group: "apps",
            name: "Steam",
            detail: "Cache des pages du magasin (tes jeux ne bougent pas)",
            paths: join(&local, "Steam/htmlcache"),
            min_age: Duration::ZERO,
            processes: &[("steam.exe", "Steam")],
            needs_admin: false,
        },
        Target {
            id: "npm_cache",
            group: "apps",
            name: "Cache npm",
            detail: "Paquets JavaScript téléchargés, retéléchargés au besoin",
            paths: join(&local, "npm-cache"),
            min_age: Duration::ZERO,
            processes: &[],
            needs_admin: false,
        },
        Target {
            id: "chrome",
            group: "browsers",
            name: "Google Chrome",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: chromium_caches(local.as_ref().map(|l| l.join("Google/Chrome/User Data"))),
            min_age: Duration::ZERO,
            processes: &[("chrome.exe", "Chrome")],
            needs_admin: false,
        },
        Target {
            id: "edge",
            group: "browsers",
            name: "Microsoft Edge",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: chromium_caches(local.as_ref().map(|l| l.join("Microsoft/Edge/User Data"))),
            min_age: Duration::ZERO,
            processes: &[("msedge.exe", "Edge")],
            needs_admin: false,
        },
        Target {
            id: "brave",
            group: "browsers",
            name: "Brave",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: chromium_caches(local.as_ref().map(|l| l.join("BraveSoftware/Brave-Browser/User Data"))),
            min_age: Duration::ZERO,
            processes: &[("brave.exe", "Brave")],
            needs_admin: false,
        },
        Target {
            id: "firefox",
            group: "browsers",
            name: "Firefox",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: firefox_caches(&local),
            min_age: Duration::ZERO,
            processes: &[("firefox.exe", "Firefox")],
            needs_admin: false,
        },
    ]
}

fn running_process(sys: &sysinfo::System, processes: &[(&str, &str)]) -> Option<String> {
    processes
        .iter()
        .find(|(exe, _)| sys.processes_by_exact_name(std::ffi::OsStr::new(exe)).next().is_some())
        .map(|(_, label)| label.to_string())
}

fn existing(paths: &[PathBuf]) -> impl Iterator<Item = &Path> {
    paths.iter().map(PathBuf::as_path).filter(|p| p.is_dir())
}

/// `progress` reçoit le nom de chaque élément au moment où il est analysé.
pub fn scan(progress: &dyn Fn(&str)) -> Vec<JunkItem> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut items: Vec<JunkItem> = targets()
        .into_iter()
        .filter_map(|t| {
            progress(t.name);
            let mut bytes = 0;
            let mut files = 0;
            let mut found = false;
            for p in existing(&t.paths) {
                found = true;
                let (b, f) = dir_size(p, t.min_age);
                bytes += b;
                files += f;
            }
            // Un navigateur ou une appli pas installée n'apparaît pas.
            if !found {
                return None;
            }
            Some(JunkItem {
                id: t.id.into(),
                group: t.group.into(),
                name: t.name.into(),
                detail: t.detail.into(),
                bytes,
                files,
                running: running_process(&sys, t.processes),
                needs_admin: t.needs_admin,
            })
        })
        .collect();

    progress("Corbeille");
    if let Some((bytes, files)) = recycle_bin::query() {
        items.push(JunkItem {
            id: "recycle_bin".into(),
            group: "trash".into(),
            name: "Corbeille".into(),
            detail: "Tous les disques".into(),
            bytes,
            files,
            running: None,
            needs_admin: false,
        });
    }
    items
}

/// Dossiers de Windows vidés par l'appli : les seuls hors de ton dossier personnel qu'elle accepte
/// de toucher en administrateur. Comparaison exacte : un TEMP détourné vers C:\Windows\System32
/// ne passe pas pour autant.
const WINDOWS_TARGETS: &[&str] = &[
    "Temp",
    "SoftwareDistribution/Download",
    "ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache",
];
const PROGRAM_DATA_TARGETS: &[&str] = &["Microsoft/Windows/WER/ReportArchive", "Microsoft/Windows/WER/ReportQueue"];

fn program_data() -> PathBuf {
    PathBuf::from(format!(r"{}\ProgramData", garde::system_drive()))
}

fn is_windows_target(path: &Path) -> bool {
    let (windir, program_data) = (garde::windows_dir(), program_data());
    WINDOWS_TARGETS.iter().any(|rel| windir.join(rel) == path)
        || PROGRAM_DATA_TARGETS.iter().any(|rel| program_data.join(rel) == path)
}

pub fn clean(ids: &[String], progress: &dyn Fn(&str)) -> CleanReport {
    let mut report = CleanReport::default();
    for t in targets().into_iter().filter(|t| ids.iter().any(|id| id == t.id)) {
        progress(t.name);
        // Les chemins de ton compte viennent de variables (TEMP, LOCALAPPDATA...) qu'un programme peut
        // détourner vers C:\Windows : en administrateur, ils doivent être dans ton dossier personnel.
        for p in existing(&t.paths).filter(|p| is_windows_target(p) || garde::user_dir_allowed(p)) {
            let (freed, removed, skipped) = clean_dir(p, t.min_age);
            report.freed += freed;
            report.removed += removed;
            report.skipped += skipped;
        }
    }
    if ids.iter().any(|id| id == "recycle_bin") {
        progress("Corbeille");
        if let Some((bytes, files)) = recycle_bin::query() {
            if recycle_bin::empty() {
                report.freed += bytes;
                report.removed += files;
            }
        }
    }
    report
}

#[cfg(windows)]
mod recycle_bin {
    use windows_sys::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
        SHQUERYRBINFO,
    };

    pub fn query() -> Option<(u64, u64)> {
        let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
        // Chemin nul : toutes les corbeilles de tous les disques.
        let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
        (hr >= 0).then(|| (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
    }

    pub fn empty() -> bool {
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) };
        hr >= 0
    }
}

#[cfg(not(windows))]
mod recycle_bin {
    pub fn query() -> Option<(u64, u64)> {
        None
    }
    pub fn empty() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_windows_folders_pass_as_windows_targets() {
        let windir = garde::windows_dir();
        assert!(is_windows_target(&windir.join("Temp")));
        assert!(is_windows_target(&windir.join("SoftwareDistribution/Download")));
        assert!(!is_windows_target(&windir));
        assert!(!is_windows_target(&windir.join("System32")));
        assert!(!is_windows_target(&windir.join("Temp/../System32")));
        assert!(!is_windows_target(&program_data()));
    }
}
