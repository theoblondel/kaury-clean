//! Applis lancées au démarrage de Windows.
//!
//! On fait exactement comme le Gestionnaire des tâches : l'entrée de démarrage reste en place et on
//! note seulement « activée » ou « désactivée » dans la clé StartupApproved. Rien n'est supprimé,
//! donc tout se réactive d'un clic.

use serde::Serialize;

#[derive(Serialize)]
pub struct StartupApp {
    id: String,
    name: String,
    command: String,
    /// "user" : ton compte, modifiable. "machine" : tous les comptes, demande les droits administrateur.
    scope: String,
    enabled: bool,
}

#[cfg(windows)]
mod imp {
    use super::StartupApp;
    use std::path::PathBuf;
    use winreg::enums::*;
    use winreg::{RegKey, RegValue, HKEY};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

    enum Kind {
        /// Une clé Run du registre.
        Registry(&'static str),
        /// Un dossier Démarrage rempli de raccourcis, donné par sa variable d'environnement de base.
        Folder(&'static str, &'static str),
    }

    struct Source {
        id: &'static str,
        hive: HKEY,
        kind: Kind,
        approved: &'static str,
        scope: &'static str,
    }

    const STARTUP_FOLDER: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";

    const SOURCES: [Source; 5] = [
        Source { id: "hkcu", hive: HKEY_CURRENT_USER, kind: Kind::Registry(RUN), approved: "Run", scope: "user" },
        Source { id: "hklm", hive: HKEY_LOCAL_MACHINE, kind: Kind::Registry(RUN), approved: "Run", scope: "machine" },
        Source { id: "hklm32", hive: HKEY_LOCAL_MACHINE, kind: Kind::Registry(RUN32), approved: "Run32", scope: "machine" },
        Source {
            id: "folder",
            hive: HKEY_CURRENT_USER,
            kind: Kind::Folder("APPDATA", STARTUP_FOLDER),
            approved: "StartupFolder",
            scope: "user",
        },
        Source {
            id: "commonfolder",
            hive: HKEY_LOCAL_MACHINE,
            kind: Kind::Folder("ProgramData", STARTUP_FOLDER),
            approved: "StartupFolder",
            scope: "machine",
        },
    ];

    fn is_enabled(approved: &Option<RegKey>, name: &str) -> bool {
        // Premier octet pair (02, 06) = activée, impair (03) = désactivée. Pas de valeur = activée.
        approved
            .as_ref()
            .and_then(|k| k.get_raw_value(name).ok())
            .and_then(|v| v.bytes.first().copied())
            .map(|b| b % 2 == 0)
            .unwrap_or(true)
    }

    fn utf16(bytes: &[u8]) -> String {
        let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&wide).trim_end_matches('\0').to_string()
    }

    fn folder(base: &str, rel: &str) -> Option<PathBuf> {
        std::env::var_os(base).map(|b| PathBuf::from(b).join(rel))
    }

    /// Les entrées d'une source : (nom de la valeur StartupApproved, nom affiché, commande).
    fn entries(src: &Source) -> Vec<(String, String, String)> {
        match src.kind {
            Kind::Registry(run) => RegKey::predef(src.hive)
                .open_subkey_with_flags(run, KEY_READ)
                .map(|k| {
                    k.enum_values()
                        .flatten()
                        .map(|(name, value)| (name.clone(), name, utf16(&value.bytes)))
                        .collect()
                })
                .unwrap_or_default(),
            Kind::Folder(base, rel) => {
                let Some(dir) = folder(base, rel) else { return vec![] };
                let Ok(read) = std::fs::read_dir(&dir) else { return vec![] };
                read.flatten()
                    .filter(|e| e.path().is_file() && e.file_name() != "desktop.ini")
                    .map(|e| {
                        let file = e.file_name().to_string_lossy().into_owned();
                        let shown = e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(file.clone());
                        (file, shown, e.path().display().to_string())
                    })
                    .collect()
            }
        }
    }

    pub fn list() -> Vec<StartupApp> {
        let mut apps = vec![];
        for src in &SOURCES {
            let approved = RegKey::predef(src.hive)
                .open_subkey_with_flags(format!(r"{APPROVED}\{}", src.approved), KEY_READ)
                .ok();
            for (key, name, command) in entries(src) {
                apps.push(StartupApp {
                    id: format!("{}|{key}", src.id),
                    enabled: is_enabled(&approved, &key),
                    name,
                    command,
                    scope: src.scope.into(),
                });
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    pub fn set(id: &str, enabled: bool) -> Result<(), String> {
        let (src_id, key) = id.split_once('|').ok_or(crate::langue::tr("Appli inconnue", "Unknown app"))?;
        let src = SOURCES.iter().find(|s| s.id == src_id).ok_or(crate::langue::tr("Appli inconnue", "Unknown app"))?;
        // On vérifie que l'entrée existe vraiment avant d'écrire quoi que ce soit.
        if !entries(src).iter().any(|(k, _, _)| k == key) {
            return Err(crate::langue::tr("Cette appli n'est plus dans la liste de démarrage", "This app is no longer in the startup list").into());
        }

        let mut bytes = vec![0u8; 12];
        if enabled {
            bytes[0] = 2;
        } else {
            bytes[0] = 3;
            // Les 8 derniers octets : date de désactivation (FILETIME), comme le Gestionnaire des tâches.
            let unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let filetime = (unix.as_secs() + 11_644_473_600) * 10_000_000 + u64::from(unix.subsec_nanos() / 100);
            bytes[4..].copy_from_slice(&filetime.to_le_bytes());
        }

        let admin = crate::langue::tr("Droits administrateur nécessaires pour cette appli", "Admin rights are required for this app").to_string();
        let (approved, _) = RegKey::predef(src.hive)
            .create_subkey_with_flags(format!(r"{APPROVED}\{}", src.approved), KEY_SET_VALUE)
            .map_err(|_| admin.clone())?;
        approved.set_raw_value(key, &RegValue { bytes, vtype: REG_BINARY }).map_err(|_| admin)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::StartupApp;
    pub fn list() -> Vec<StartupApp> {
        vec![]
    }
    pub fn set(_id: &str, _enabled: bool) -> Result<(), String> {
        Err(crate::langue::tr("Disponible uniquement sur Windows", "Only available on Windows").into())
    }
}

pub use imp::{list, set};
