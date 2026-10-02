//! Ce qui prend de la place sur le disque de Windows. Lecture seule : rien ici n'efface, ne déplace
//! ni ne modifie quoi que ce soit. C'est une carte, pour savoir où trier soi-même.

use crate::langue::tr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use walkdir::WalkDir;

use crate::files::cancelled;
use crate::garde;

#[derive(Serialize)]
pub struct SpaceEntry {
    name: String,
    path: String,
    bytes: u64,
    kind: &'static str,
    note: String,
}

#[derive(Serialize)]
pub struct SpaceGroup {
    kind: &'static str,
    label: &'static str,
    bytes: u64,
}

#[derive(Serialize)]
pub struct SpaceReport {
    total: u64,
    used: u64,
    groups: Vec<SpaceGroup>,
    top: Vec<SpaceEntry>,
}

/// Dossiers affichés par la dernière analyse : les seuls que « Ouvrir » accepte.
static SHOWN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

const TOP: usize = 24;

/// Taille d'un dossier, sans suivre les liens ni les jonctions. S'arrête si tu cliques sur « Arrêter ».
fn size_of(dir: &Path) -> u64 {
    WalkDir::new(dir)
        .follow_root_links(false)
        .min_depth(1)
        .into_iter()
        .take_while(|_| !cancelled())
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| !cloud_only(m))
        .map(|m| m.len())
        .sum()
}

/// Un fichier OneDrive « disponible en ligne » ne prend pas de place sur le disque : on ne le compte pas.
#[cfg(windows)]
fn cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    meta.file_attributes() & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS) != 0
}

#[cfg(not(windows))]
fn cloud_only(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Sous-dossiers réels d'un dossier (les liens et jonctions, comme « Application Data », sont ignorés).
fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    entries.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).map(|e| e.path()).collect()
}

/// Une phrase d'orientation pour les gros dossiers qu'on croise souvent. Jamais une invitation à effacer.
pub fn note_for(name: &str, kind: &str) -> String {
    let n = name.to_lowercase();
    let known: &[(&[&str], &str)] = &[
        (&["packages"], tr("Applis du Microsoft Store et leurs données (WhatsApp, Spotify, Claude…)", "Microsoft Store apps and their data (WhatsApp, Spotify, Claude…)")),
        (&["docker"], tr("Disques de Docker : à gérer depuis Docker Desktop", "Docker disks: manage them from Docker Desktop")),
        (&[".minecraft", "modrinthapp", "prismlauncher", "curseforge"], tr("Mondes, modpacks et versions de Minecraft", "Minecraft worlds, modpacks and versions")),
        (&["steam", "steamlibrary"], tr("Tes jeux Steam : à désinstaller depuis Steam", "Your Steam games: uninstall them from Steam")),
        (&["epic games"], tr("Tes jeux Epic : à désinstaller depuis le launcher Epic", "Your Epic games: uninstall them from the Epic launcher")),
        (&["google"], tr("Chrome : profils, extensions et données des sites. Le cache se vide dans Nettoyage", "Chrome: profiles, extensions and site data. The cache is cleared in Cleanup")),
        (&["microsoft"], tr("Données des applis Microsoft (Edge, Office, Teams…)", "Microsoft app data (Edge, Office, Teams…)")),
        (&["claude"], tr("Claude : l'appli et sa machine virtuelle", "Claude: the app and its virtual machine")),
        (&["nvidia corporation", "nvidia"], tr("Pilotes et appli NVIDIA. Les installeurs se vident dans Nettoyage", "NVIDIA drivers and app. The installers are cleared in Cleanup")),
        (&["adobe"], tr("Applis Adobe et leurs caches", "Adobe apps and their caches")),
        (&["uv"], tr("Python (uv). Son cache se vide dans Nettoyage › Applications", "Python (uv). Its cache is cleared in Cleanup › Applications")),
        (&["npm-cache"], tr("Cache npm. Il se vide dans Nettoyage › Applications", "npm cache. It's cleared in Cleanup › Applications")),
        (&["temp"], tr("Fichiers temporaires. Ils se vident dans Nettoyage", "Temporary files. They're cleared in Cleanup")),
        (&["onedrive"], tr("Tes fichiers synchronisés avec OneDrive", "Your files synced with OneDrive")),
        (&["windowsapps"], tr("Applis du Microsoft Store (lisible seulement en administrateur)", "Microsoft Store apps (readable only as administrator)")),
    ];
    if let Some((_, note)) = known.iter().find(|(names, _)| names.contains(&n.as_str())) {
        return (*note).into();
    }
    match kind {
        "files" => tr("Tes fichiers : « Gros fichiers » et « Doublons » t'aident à trier", "Your files: \"Large files\" and \"Duplicates\" help you sort them").into(),
        "apps" => tr("Données d'une appli : réglages, caches, parfois ton travail", "An app's data: settings, caches, sometimes your work").into(),
        "programs" => tr("Un programme installé : à désinstaller depuis Applications s'il ne sert plus", "An installed program: uninstall it from Applications if you no longer use it").into(),
        _ => String::new(),
    }
}

/// Quelle famille : tes fichiers, données d'applis ou programmes.
pub fn kind_of_profile_child(name: &str) -> &'static str {
    if name.starts_with('.') {
        "apps"
    } else {
        "files"
    }
}

pub fn scan(total: u64, used: u64, progress: &dyn Fn(&str)) -> SpaceReport {
    let drive = garde::system_drive();
    let mut places: Vec<(PathBuf, &'static str)> = vec![];

    if let Some(profile) = garde::profile_dir() {
        for c in children(&profile) {
            let name = c.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if name.eq_ignore_ascii_case("AppData") {
                for sub in ["Local", "Roaming", "LocalLow"] {
                    places.extend(children(&c.join(sub)).into_iter().map(|p| (p, "apps")));
                }
            } else {
                places.push((c, kind_of_profile_child(&name)));
            }
        }
    }
    if cfg!(target_os = "macos") {
        // macOS : les applis dans /Applications, leurs données dans ~/Library (déjà parcourue plus haut
        // comme un dossier de ton compte, on la détaille ici).
        let _ = &drive;
        places.retain(|(p, _)| p.file_name().is_none_or(|n| n != "Library"));
        if let Some(home) = garde::profile_dir() {
            for sub in ["Application Support", "Caches", "Containers", "Developer", "Group Containers"] {
                places.extend(children(&home.join("Library").join(sub)).into_iter().map(|p| (p, "apps")));
            }
        }
        places.extend(children(&PathBuf::from("/Applications")).into_iter().map(|p| (p, "programs")));
    } else {
        places.extend(children(&PathBuf::from(format!(r"{drive}\ProgramData"))).into_iter().map(|p| (p, "apps")));
        for pf in ["Program Files", "Program Files (x86)"] {
            places.extend(children(&PathBuf::from(format!(r"{drive}\{pf}"))).into_iter().map(|p| (p, "programs")));
        }
    }

    let mut entries: Vec<SpaceEntry> = vec![];
    for (path, kind) in places {
        if cancelled() {
            break;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        progress(&name);
        let bytes = size_of(&path);
        if bytes == 0 {
            continue;
        }
        entries.push(SpaceEntry { note: note_for(&name, kind), name, path: path.to_string_lossy().into_owned(), bytes, kind });
    }

    let sum = |k: &str| entries.iter().filter(|e| e.kind == k).map(|e| e.bytes).sum::<u64>();
    let (files, apps, programs) = (sum("files"), sum("apps"), sum("programs"));
    let groups = vec![
        SpaceGroup { kind: "files", label: tr("Tes fichiers", "Your files"), bytes: files },
        SpaceGroup { kind: "apps", label: tr("Données d'applis", "App data"), bytes: apps },
        SpaceGroup { kind: "programs", label: tr("Programmes", "Programs"), bytes: programs },
        // Le reste de l'espace utilisé : Windows, fichier d'échange, mise en veille prolongée, points de restauration.
        SpaceGroup { kind: "system", label: if cfg!(target_os = "macos") { tr("macOS et le reste", "macOS and the rest") } else { tr("Windows et le reste", "Windows and the rest") }, bytes: used.saturating_sub(files + apps + programs) },
    ];

    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    entries.truncate(TOP);
    if let Ok(mut shown) = SHOWN.lock() {
        *shown = entries.iter().map(|e| PathBuf::from(&e.path)).collect();
    }
    SpaceReport { total, used, groups, top: entries }
}

/// Ouvre un dossier de la dernière analyse dans l'Explorateur. Rien d'autre n'est accepté.
pub fn open(path: &str) -> Result<(), String> {
    let wanted = PathBuf::from(path);
    let allowed = SHOWN.lock().map(|s| s.contains(&wanted)).unwrap_or(false);
    if !allowed || !wanted.is_dir() {
        return Err(tr("Dossier introuvable", "Folder not found").into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(garde::windows_program("explorer.exe"))
            .raw_arg(format!("\"{}\"", wanted.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open").arg(&wanted).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    Err(tr("Disponible uniquement sur Windows", "Only available on Windows").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_without_following_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.path().join("sub/b.bin"), vec![0u8; 50]).unwrap();
        assert_eq!(size_of(dir.path()), 150);
    }

    #[test]
    fn sorts_profile_folders() {
        assert_eq!(kind_of_profile_child("Videos"), "files");
        assert_eq!(kind_of_profile_child(".gradle"), "apps");
    }

    #[test]
    fn notes_never_suggest_deleting_data() {
        for name in ["Packages", "Docker", ".minecraft", "Google", "Claude", "Videos", "SomeApp"] {
            for kind in ["files", "apps", "programs"] {
                let note = note_for(name, kind).to_lowercase();
                assert!(!note.contains("supprime") && !note.contains("efface"), "{name} : {note}");
            }
        }
    }

    #[test]
    fn opens_only_folders_from_the_last_scan() {
        assert!(open(r"C:\Windows\System32").is_err());
    }
}
