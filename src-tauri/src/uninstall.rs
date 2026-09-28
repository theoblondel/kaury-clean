//! Désinstaller des applis. On lance le désinstalleur officiel de chaque appli, exactement comme
//! « Paramètres > Applications » de Windows : jamais de suppression de dossier à la main.

use serde::Serialize;

#[derive(Serialize)]
pub struct InstalledApp {
    id: String,
    name: String,
    publisher: String,
    version: String,
    bytes: u64,
    /// Date d'installation au format AAAAMMJJ, vide si inconnue.
    installed: String,
}

/// Sépare une ligne de commande de désinstallation en (programme, arguments).
/// `"C:\Program Files\X\uninst.exe" /S` → (`C:\Program Files\X\uninst.exe`, `/S`)
/// `MsiExec.exe /X{GUID}` → (`MsiExec.exe`, `/X{GUID}`)
/// `C:\Program Files\X\uninst.exe /S` (sans guillemets) → coupé juste après `.exe`
#[cfg_attr(not(windows), allow(dead_code))]
pub fn split_command(cmd: &str) -> (String, String) {
    let cmd = cmd.trim();
    if let Some(rest) = cmd.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return (rest[..end].to_string(), rest[end + 1..].trim().to_string());
        }
    }
    // Minuscules ASCII seulement : les positions en octets restent les mêmes que dans `cmd`.
    if let Some(pos) = cmd.to_ascii_lowercase().find(".exe") {
        let cut = pos + 4;
        return (cmd[..cut].to_string(), cmd[cut..].trim().to_string());
    }
    match cmd.split_once(' ') {
        Some((exe, args)) => (exe.to_string(), args.trim().to_string()),
        None => (cmd.to_string(), String::new()),
    }
}

/// Beaucoup d'applis MSI déclarent `MsiExec.exe /I{GUID}`, qui ouvre l'écran « Modifier / Réparer ».
/// On demande directement la désinstallation (`/X{GUID}`).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn msi_uninstall_args(exe: &str, args: &str) -> String {
    // Découpage à la main : le chemin est un chemin Windows, même quand les tests tournent ailleurs.
    let name = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_ascii_lowercase();
    let is_msiexec = name == "msiexec.exe" || name == "msiexec";
    match args.get(..2) {
        Some(flag) if is_msiexec && flag.eq_ignore_ascii_case("/i") => format!("/X{}", &args[2..]),
        _ => args.to_string(),
    }
}

#[cfg(windows)]
mod imp {
    use super::{msi_uninstall_args, split_command, InstalledApp};
    use std::collections::HashSet;
    use winreg::enums::*;
    use winreg::{RegKey, HKEY};

    const SOURCES: [(&str, HKEY, &str); 3] = [
        ("hklm", HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("hklm32", HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("hkcu", HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];

    fn text(key: &RegKey, name: &str) -> String {
        key.get_value::<String, _>(name).unwrap_or_default().trim().to_string()
    }

    fn open(src: &str, sub: &str) -> Option<RegKey> {
        let (_, hive, path) = SOURCES.iter().find(|s| s.0 == src)?;
        RegKey::predef(*hive).open_subkey_with_flags(format!(r"{path}\{sub}"), KEY_READ).ok()
    }

    /// Une entrée qui correspond à une vraie appli, pas à une mise à jour ou un composant caché.
    fn visible(key: &RegKey) -> bool {
        !text(key, "DisplayName").is_empty()
            && !text(key, "UninstallString").is_empty()
            && key.get_value::<u32, _>("SystemComponent").unwrap_or(0) != 1
            && text(key, "ParentKeyName").is_empty()
            && !matches!(text(key, "ReleaseType").as_str(), "Update" | "Hotfix" | "Security Update")
    }

    pub fn list() -> Vec<InstalledApp> {
        let mut seen = HashSet::new();
        let mut apps = vec![];
        for (src, hive, path) in SOURCES {
            let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ) else { continue };
            for sub in root.enum_keys().flatten() {
                let Ok(key) = root.open_subkey_with_flags(&sub, KEY_READ) else { continue };
                if !visible(&key) {
                    continue;
                }
                let name = text(&key, "DisplayName");
                let version = text(&key, "DisplayVersion");
                // La même appli apparaît parfois en 32 et 64 bits.
                if !seen.insert((name.to_lowercase(), version.clone())) {
                    continue;
                }
                apps.push(InstalledApp {
                    id: format!("{src}|{sub}"),
                    publisher: text(&key, "Publisher"),
                    bytes: u64::from(key.get_value::<u32, _>("EstimatedSize").unwrap_or(0)) * 1024,
                    installed: text(&key, "InstallDate"),
                    name,
                    version,
                });
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    pub fn uninstall(id: &str) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let (src, sub) = id.split_once('|').ok_or(crate::langue::tr("Appli inconnue", "Unknown app"))?;
        // Les applis « de ton compte » sont inscrites à un endroit que n'importe quel programme peut
        // modifier. Lancer leur désinstalleur en administrateur offrirait ces droits à un imposteur,
        // sans même la fenêtre de confirmation de Windows.
        if src == "hkcu" && crate::elevation::is_elevated() {
            return Err(crate::langue::tr(
                "Par sécurité, cette appli (installée pour ton compte seulement) ne se désinstalle pas depuis Kaury Clean en mode administrateur. Rouvre Kaury Clean normalement, ou passe par Paramètres › Applications.",
                "For safety, this app (installed for your account only) can't be uninstalled from Kaury Clean in administrator mode. Open Kaury Clean normally, or go through Settings › Apps.",
            )
            .into());
        }
        let key = open(src, sub).filter(visible).ok_or(crate::langue::tr("Cette appli n'est plus installée", "This app is no longer installed"))?;
        // La commande vient du registre, jamais de l'interface.
        let (exe, args) = split_command(&text(&key, "UninstallString"));
        let exe = if exe.eq_ignore_ascii_case("msiexec.exe") || exe.eq_ignore_ascii_case("msiexec") {
            crate::garde::windows_program("msiexec.exe").to_string_lossy().into_owned()
        } else {
            exe
        };
        let args = msi_uninstall_args(&exe, &args);

        let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<u16>>();
        let (verb, file, params) = (wide("open"), wide(&exe), wide(&args));
        // ShellExecute (et pas Command) : Windows affiche lui-même la demande d'administrateur si le désinstalleur en a besoin.
        let result = unsafe {
            ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), SW_SHOWNORMAL)
        };
        if result as isize > 32 {
            Ok(())
        } else {
            Err(crate::langue::tr("Le désinstalleur n'a pas pu démarrer", "The uninstaller couldn't start").into())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::InstalledApp;
    pub fn list() -> Vec<InstalledApp> {
        vec![]
    }
    pub fn uninstall(_id: &str) -> Result<(), String> {
        Err(crate::langue::tr("Disponible uniquement sur Windows", "Only available on Windows").into())
    }
}

pub use imp::{list, uninstall};

#[cfg(test)]
mod tests {
    use super::{msi_uninstall_args, split_command};

    #[test]
    fn turns_msi_repair_into_uninstall() {
        assert_eq!(msi_uninstall_args("MsiExec.exe", "/I{1234-ABCD}"), "/X{1234-ABCD}");
        assert_eq!(msi_uninstall_args(r"C:\Windows\System32\msiexec.exe", "/i{AB} /qb"), "/X{AB} /qb");
        assert_eq!(msi_uninstall_args("MsiExec.exe", "/X{1234}"), "/X{1234}");
        assert_eq!(msi_uninstall_args(r"C:\App\unins000.exe", "/I"), "/I");
    }

    #[test]
    fn splits_quoted_commands() {
        assert_eq!(
            split_command(r#""C:\Program Files\Figma\Uninstall Figma.exe" /S --force"#),
            (r"C:\Program Files\Figma\Uninstall Figma.exe".into(), "/S --force".into())
        );
    }

    #[test]
    fn splits_msiexec_and_unquoted_paths() {
        assert_eq!(split_command("MsiExec.exe /X{1234-ABCD}"), ("MsiExec.exe".into(), "/X{1234-ABCD}".into()));
        assert_eq!(
            split_command(r"C:\Program Files\Some App\unins000.exe /SILENT"),
            (r"C:\Program Files\Some App\unins000.exe".into(), "/SILENT".into())
        );
        assert_eq!(split_command(r"C:\Tools\remove.exe"), (r"C:\Tools\remove.exe".into(), String::new()));
    }
}
