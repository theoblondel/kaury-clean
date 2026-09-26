//! Mémoire vive (RAM). Windows gère déjà très bien la RAM tout seul : les boutons « nettoyer la RAM »
//! ne font que la vider pour qu'elle se remplisse aussitôt. Ce qui accélère vraiment un PC qui rame,
//! c'est de fermer les applis qui en prennent trop. C'est ce que fait ce module.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::process::Command;

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

#[derive(Serialize)]
pub struct MemoryStatus {
    total: u64,
    used: u64,
    apps: Vec<AppMemory>,
}

#[derive(Serialize)]
pub struct AppMemory {
    /// Nom du .exe, qui sert aussi d'identifiant pour fermer l'appli.
    exe: String,
    name: String,
    bytes: u64,
    /// Nombre de processus (Chrome en lance des dizaines).
    processes: u64,
}

/// Processus de Windows (et de Kaury Clean) qu'on ne propose jamais de fermer.
const PROTECTED: &[&str] = &[
    "system", "idle", "registry", "memory compression", "secure system", "smss.exe", "csrss.exe", "wininit.exe",
    "winlogon.exe", "services.exe", "lsass.exe", "lsaiso.exe", "svchost.exe", "dwm.exe", "explorer.exe",
    "fontdrvhost.exe", "sihost.exe", "ctfmon.exe", "runtimebroker.exe", "searchhost.exe", "searchindexer.exe",
    "startmenuexperiencehost.exe", "shellexperiencehost.exe", "textinputhost.exe", "audiodg.exe", "spoolsv.exe",
    "conhost.exe", "taskhostw.exe", "msmpeng.exe", "nissrv.exe", "securityhealthservice.exe",
    "securityhealthsystray.exe", "smartscreen.exe", "wudfhost.exe", "dllhost.exe", "applicationframehost.exe",
    "systemsettings.exe", "lockapp.exe", "useroobebroker.exe", "widgets.exe", "phoneexperiencehost.exe",
    "kaury-clean.exe", "msedgewebview2.exe",
];

fn protected(exe: &str) -> bool {
    PROTECTED.contains(&exe.to_lowercase().as_str())
}

/// « chrome.exe » → « Chrome », « AfterFX.exe » → « AfterFX ».
fn pretty(exe: &str) -> String {
    let stem = exe.strip_suffix(".exe").or_else(|| exe.strip_suffix(".EXE")).unwrap_or(exe);
    let mut chars = stem.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn system() -> System {
    System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram())
            .with_processes(ProcessRefreshKind::nothing().with_memory()),
    )
}

pub fn status() -> MemoryStatus {
    let sys = system();
    let mut by_exe: HashMap<String, AppMemory> = HashMap::new();
    for p in sys.processes().values() {
        let exe = p.name().to_string_lossy().into_owned();
        if exe.is_empty() || protected(&exe) {
            continue;
        }
        let app = by_exe.entry(exe.to_lowercase()).or_insert_with(|| AppMemory {
            name: pretty(&exe),
            exe: exe.clone(),
            bytes: 0,
            processes: 0,
        });
        app.bytes += p.memory();
        app.processes += 1;
    }
    let mut apps: Vec<AppMemory> = by_exe.into_values().filter(|a| a.bytes >= 50 * 1024 * 1024).collect();
    apps.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    apps.truncate(20);
    MemoryStatus { total: sys.total_memory(), used: sys.used_memory(), apps }
}

fn still_running(exe: &str) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let running = sys.processes_by_exact_name(OsStr::new(exe)).next().is_some();
    running
}

/// Ferme une appli. D'abord poliment (comme la croix de la fenêtre, l'appli peut te proposer
/// d'enregistrer) ; avec `force`, immédiatement. Renvoie `true` si l'appli est bien fermée.
pub fn close(exe: &str, force: bool) -> Result<bool, String> {
    if protected(exe) || !still_running(exe) {
        return Err("Cette appli ne peut pas être fermée d'ici".into());
    }
    let mut cmd = Command::new(crate::garde::windows_program("taskkill.exe"));
    if force {
        cmd.arg("/F");
    }
    cmd.args(["/T", "/IM", exe]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW : pas de fenêtre noire qui clignote
    }
    let _ = cmd.output().map_err(|e| e.to_string())?;
    // On laisse quelques secondes à l'appli pour se fermer proprement.
    for _ in 0..10 {
        if !still_running(exe) {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_windows_and_itself() {
        assert!(protected("svchost.exe"));
        assert!(protected("Explorer.EXE"));
        assert!(protected("kaury-clean.exe"));
        assert!(!protected("chrome.exe"));
    }

    #[test]
    fn prettifies_exe_names() {
        assert_eq!(pretty("chrome.exe"), "Chrome");
        assert_eq!(pretty("AfterFX.exe"), "AfterFX");
    }
}
