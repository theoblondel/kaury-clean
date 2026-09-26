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
    use winreg::enums::*;
    use winreg::{RegKey, RegValue, HKEY};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

    /// Chaque source : (id, ruche, clé Run, sous-clé StartupApproved, portée).
    const SOURCES: [(&str, HKEY, &str, &str, &str); 3] = [
        ("hkcu", HKEY_CURRENT_USER, RUN, "Run", "user"),
        ("hklm", HKEY_LOCAL_MACHINE, RUN, "Run", "machine"),
        ("hklm32", HKEY_LOCAL_MACHINE, RUN32, "Run32", "machine"),
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

    pub fn list() -> Vec<StartupApp> {
        let mut apps = vec![];
        for (src, hive, run, approved_sub, scope) in SOURCES {
            let root = RegKey::predef(hive);
            let Ok(run_key) = root.open_subkey_with_flags(run, KEY_READ) else { continue };
            let approved = root.open_subkey_with_flags(format!(r"{APPROVED}\{approved_sub}"), KEY_READ).ok();
            for (name, value) in run_key.enum_values().flatten() {
                let command = String::from_utf16_lossy(
                    &value.bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>(),
                )
                .trim_end_matches('\0')
                .to_string();
                apps.push(StartupApp {
                    id: format!("{src}|{name}"),
                    enabled: is_enabled(&approved, &name),
                    name,
                    command,
                    scope: scope.into(),
                });
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    pub fn set(id: &str, enabled: bool) -> Result<(), String> {
        let (src, name) = id.split_once('|').ok_or("Appli inconnue")?;
        let (_, hive, run, approved_sub, _) =
            SOURCES.into_iter().find(|s| s.0 == src).ok_or("Appli inconnue")?;
        let root = RegKey::predef(hive);
        // On vérifie que l'entrée existe vraiment avant d'écrire quoi que ce soit.
        root.open_subkey_with_flags(run, KEY_READ)
            .and_then(|k| k.get_raw_value(name))
            .map_err(|_| "Cette appli n'est plus dans la liste de démarrage".to_string())?;

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

        let (key, _) = root
            .create_subkey_with_flags(format!(r"{APPROVED}\{approved_sub}"), KEY_SET_VALUE)
            .map_err(|_| "Droits administrateur nécessaires pour cette appli".to_string())?;
        key.set_raw_value(name, &RegValue { bytes, vtype: REG_BINARY })
            .map_err(|_| "Droits administrateur nécessaires pour cette appli".to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::StartupApp;
    pub fn list() -> Vec<StartupApp> {
        vec![]
    }
    pub fn set(_id: &str, _enabled: bool) -> Result<(), String> {
        Err("Disponible uniquement sur Windows".into())
    }
}

pub use imp::{list, set};
