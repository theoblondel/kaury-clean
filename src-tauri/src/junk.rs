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

/// Caches d'un navigateur ou d'une appli basée sur Chromium / WebView2, profil par profil
/// (Default, Profile 1, WV2Profile_...). Seulement Cache, Code Cache et GPUCache : les connexions,
/// l'historique et les données des sites (IndexedDB, Service Worker...) ne sont jamais touchés.
fn chromium_caches(user_data: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(user_data) = user_data else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&user_data) else { return vec![] };
    let mut out = vec![];
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "Default" || name.starts_with("Profile ") || name.starts_with("WV2Profile_") {
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                out.push(entry.path().join(sub));
            }
        }
    }
    out.push(user_data.join("ShaderCache"));
    out.push(user_data.join("GrShaderCache"));
    out
}

/// Caches d'une appli Electron (VS Code, Slack...) : jamais ses réglages ni ses données.
fn electron_caches(base: &Option<PathBuf>, app: &str, extra: &[&str]) -> Vec<PathBuf> {
    ["Cache", "Code Cache", "GPUCache"].iter().chain(extra).flat_map(|sub| join(base, &format!("{app}/{sub}"))).collect()
}

/// Dossier local d'une appli du Microsoft Store.
fn store_app(local: &Option<PathBuf>, package: &str, rel: &str) -> Option<PathBuf> {
    local.as_ref().map(|l| l.join("Packages").join(package).join(rel))
}

/// Sous-dossiers de `dir` dont le nom commence par `prefix` (« webcache_4430 »...).
fn prefixed(dir: Option<PathBuf>, prefix: &str) -> Vec<PathBuf> {
    let Some(dir) = dir else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&dir) else { return vec![] };
    entries.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(prefix)).map(|e| e.path()).collect()
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
    // La version du Microsoft Store range tout ailleurs.
    let spotify_store = store_app(&local, "SpotifyAB.SpotifyMusic_zpdnekdrzrea0", "LocalCache/Spotify");
    spotify.extend(spotify_store.iter().map(|s| s.join("Data")));
    spotify.extend(chromium_caches(spotify_store));

    let mut dev_python = join(&local, "uv/cache");
    dev_python.extend(join(&local, "pip/cache"));

    let mut code_editors = electron_caches(&roaming, "Code", &["CachedData", "CachedExtensionVSIXs"]);
    code_editors.extend(electron_caches(&roaming, "Cursor", &["CachedData", "CachedExtensionVSIXs"]));

    let mut opera = vec![];
    for base in [&local, &roaming] {
        for flavour in ["Opera Stable", "Opera GX Stable"] {
            opera.extend(chromium_caches(base.as_ref().map(|b| b.join("Opera Software").join(flavour))));
            opera.extend(join(base, &format!("Opera Software/{flavour}/Cache")));
        }
    }

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
            id: "nvidia_installers",
            group: "system",
            name: "Installeurs NVIDIA",
            detail: "Pilote et appli NVIDIA déjà installés : l'appli NVIDIA les retélécharge au besoin",
            paths: join(&program_data, "NVIDIA Corporation/NVIDIA app/UpdateFramework/ota-artifacts"),
            min_age: DAY,
            processes: &[("NVIDIA app.exe", "NVIDIA app")],
            needs_admin: true,
        },
        Target {
            id: "whatsapp",
            group: "apps",
            name: "WhatsApp",
            detail: "Cache uniquement : tes messages, tes photos et ta connexion ne bougent pas",
            paths: chromium_caches(store_app(&local, "5319275A.WhatsAppDesktop_cv1g1gvanyjgm", "LocalCache/EBWebView")),
            min_age: Duration::ZERO,
            processes: &[("WhatsApp.Root.exe", "WhatsApp"), ("WhatsApp.exe", "WhatsApp")],
            needs_admin: false,
        },
        Target {
            id: "teams",
            group: "apps",
            name: "Microsoft Teams",
            detail: "Cache uniquement : tes conversations et ta connexion ne bougent pas",
            paths: chromium_caches(store_app(&local, "MSTeams_8wekyb3d8bbwe", "LocalCache/Microsoft/MSTeams/EBWebView")),
            min_age: Duration::ZERO,
            processes: &[("ms-teams.exe", "Teams")],
            needs_admin: false,
        },
        Target {
            id: "slack",
            group: "apps",
            name: "Slack",
            detail: "Cache uniquement : tes messages et ta connexion ne bougent pas",
            paths: electron_caches(&roaming, "Slack", &[]),
            min_age: Duration::ZERO,
            processes: &[("slack.exe", "Slack")],
            needs_admin: false,
        },
        Target {
            id: "epic",
            group: "apps",
            name: "Epic Games",
            detail: "Cache des pages du magasin (tes jeux ne bougent pas)",
            paths: prefixed(local.as_ref().map(|l| l.join("EpicGamesLauncher/Saved")), "webcache"),
            min_age: Duration::ZERO,
            processes: &[("EpicGamesLauncher.exe", "Epic Games")],
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
            id: "python_cache",
            group: "apps",
            name: "Cache Python (uv, pip)",
            detail: "Paquets Python téléchargés : tes projets gardent les leurs, le reste se retélécharge au besoin",
            paths: dev_python,
            min_age: Duration::ZERO,
            processes: &[("uv.exe", "uv")],
            needs_admin: false,
        },
        Target {
            id: "yarn_cache",
            group: "apps",
            name: "Cache Yarn",
            detail: "Paquets JavaScript téléchargés, retéléchargés au besoin",
            paths: join(&local, "Yarn/Cache"),
            min_age: Duration::ZERO,
            processes: &[],
            needs_admin: false,
        },
        Target {
            id: "code_editors",
            group: "apps",
            name: "VS Code et Cursor",
            detail: "Caches et extensions déjà installées : tes réglages et tes projets ne bougent pas",
            paths: code_editors,
            min_age: Duration::ZERO,
            processes: &[("Code.exe", "VS Code"), ("Cursor.exe", "Cursor")],
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
            id: "opera",
            group: "browsers",
            name: "Opera et Opera GX",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: opera,
            min_age: Duration::ZERO,
            processes: &[("opera.exe", "Opera")],
            needs_admin: false,
        },
        Target {
            id: "vivaldi",
            group: "browsers",
            name: "Vivaldi",
            detail: "Cache uniquement : mots de passe, favoris et sessions ne bougent pas",
            paths: chromium_caches(local.as_ref().map(|l| l.join("Vivaldi/User Data"))),
            min_age: Duration::ZERO,
            processes: &[("vivaldi.exe", "Vivaldi")],
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
const PROGRAM_DATA_TARGETS: &[&str] = &[
    "Microsoft/Windows/WER/ReportArchive",
    "Microsoft/Windows/WER/ReportQueue",
    "NVIDIA Corporation/NVIDIA app/UpdateFramework/ota-artifacts",
];

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

    /// Garde-fou de la règle « que des choses inutiles » : aucune cible ne vise un dossier de
    /// données (connexions, messages, données des sites, programmes installés) ni un profil entier.
    #[test]
    fn no_target_touches_user_data() {
        const FORBIDDEN: &[&str] =
            &["indexeddb", "service worker", "local storage", "localstate", "webstorage", "cookies", "login data", "winget"];
        for t in targets() {
            for p in &t.paths {
                let p = p.to_string_lossy().replace('\\', "/").to_lowercase();
                assert!(!FORBIDDEN.iter().any(|bad| p.contains(bad)), "{} vise {p}", t.id);
                let last = p.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
                assert!(!["default", "user data", "ebwebview", "spotify"].contains(&last.as_str()), "{} vise {p}", t.id);
            }
        }
    }

    #[test]
    fn chromium_caches_only_pick_cache_folders() {
        let root = tempfile::tempdir().unwrap();
        for d in ["Default", "Profile 2", "WV2Profile_tfw", "Crashpad", "Snapshots"] {
            std::fs::create_dir(root.path().join(d)).unwrap();
        }
        let names: Vec<String> = chromium_caches(Some(root.path().to_path_buf()))
            .iter()
            .map(|p| p.strip_prefix(root.path()).unwrap().to_string_lossy().replace('\\', "/"))
            .collect();
        for n in &names {
            let last = n.rsplit('/').next().unwrap();
            assert!(["Cache", "Code Cache", "GPUCache", "ShaderCache", "GrShaderCache"].contains(&last), "{n}");
        }
        assert!(names.contains(&"WV2Profile_tfw/Cache".to_string()));
        assert!(!names.iter().any(|n| n.starts_with("Crashpad") || n.starts_with("Snapshots")));
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = targets().iter().map(|t| t.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len());
    }
}
