//! Maintenance et réparation : les outils officiels de Windows, lancés pour toi avec les bonnes options.
//! Chaque commande est fixée ici ; l'interface n'envoie que l'identifiant de la tâche.

use crate::langue::{tr, anglais};
use std::process::Command;

use serde::Serialize;

struct Task {
    id: &'static str,
    name: &'static str,
    detail: &'static str,
    /// Durée indicative affichée pendant que la tâche tourne.
    duration: &'static str,
    needs_admin: bool,
    /// Programmes lancés l'un après l'autre (programme, arguments).
    steps: &'static [(&'static str, &'static [&'static str])],
}

/// Les textes suivent la langue choisie : la liste se construit à chaque appel.
fn tasks() -> Vec<Task> {
    vec![
    Task {
        id: "restore_point",
        name: tr("Créer un point de restauration", "Create a restore point"),
        detail: tr("Une sauvegarde de l'état de Windows, pour revenir en arrière si une réparation ou une désinstallation se passe mal. Active la protection du système si elle est coupée. À faire avant les autres tâches.", "A snapshot of Windows' state, to go back if a repair or an uninstall goes wrong. Turns on system protection if it's off. Do this before the other tasks."),
        duration: tr("1 à 2 min", "1 to 2 min"),
        needs_admin: true,
        steps: &[(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Enable-ComputerRestore -Drive \"$env:SystemDrive\\\"; Checkpoint-Computer -Description 'Kaury Clean' -RestorePointType MODIFY_SETTINGS -ErrorAction Stop",
            ],
        )],
    },
    Task {
        id: "repair_windows",
        name: tr("Réparer Windows", "Repair Windows"),
        detail: tr("Vérifie et répare les fichiers système abîmés (DISM puis SFC). À faire si Windows plante, affiche des erreurs bizarres ou si des applis ne s'ouvrent plus. Il faut Internet.", "Checks and repairs damaged system files (DISM then SFC). Do this if Windows crashes, shows strange errors or apps no longer open. Requires Internet."),
        duration: tr("15 à 30 min", "15 to 30 min"),
        needs_admin: true,
        steps: &[
            ("DISM.exe", &["/Online", "/Cleanup-Image", "/RestoreHealth"]),
            ("sfc.exe", &["/scannow"]),
        ],
    },
    Task {
        id: "component_cleanup",
        name: tr("Supprimer les anciennes versions de Windows", "Remove old Windows versions"),
        detail: tr("Retire les composants remplacés par les mises à jour. Libère souvent plusieurs Go.", "Removes components replaced by updates. Often frees several GB."),
        duration: tr("5 à 15 min", "5 to 15 min"),
        needs_admin: true,
        steps: &[("DISM.exe", &["/Online", "/Cleanup-Image", "/StartComponentCleanup"])],
    },
    Task {
        id: "optimize_drive",
        name: tr("Optimiser le disque", "Optimize the drive"),
        detail: tr("Envoie TRIM à un SSD ou défragmente un disque dur, selon ton matériel. Garde le disque rapide.", "Sends TRIM to an SSD or defragments a hard drive, depending on your hardware. Keeps the drive fast."),
        duration: tr("1 à 10 min", "1 to 10 min"),
        needs_admin: true,
        steps: &[("defrag.exe", &["%SystemDrive%", "/O"])],
    },
    Task {
        id: "check_disk",
        name: tr("Vérifier le disque", "Check the drive"),
        detail: tr("Cherche les erreurs du système de fichiers sans redémarrer.", "Looks for file system errors without restarting."),
        duration: tr("2 à 10 min", "2 to 10 min"),
        needs_admin: true,
        steps: &[("chkdsk.exe", &["%SystemDrive%", "/scan"])],
    },
    Task {
        id: "hibernate_off",
        name: tr("Désactiver la veille prolongée", "Turn off hibernation"),
        detail: tr("Supprime le fichier hiberfil.sys, souvent aussi gros que la moitié de ta RAM. Utile sur un PC fixe ; sur un portable, garde-la si tu utilises la veille prolongée. Le démarrage rapide de Windows est aussi désactivé.", "Deletes hiberfil.sys, often as large as half your RAM. Useful on a desktop PC; on a laptop, keep it if you use hibernation. Windows Fast Startup is also turned off."),
        duration: tr("quelques secondes", "a few seconds"),
        needs_admin: true,
        steps: &[("powercfg.exe", &["/hibernate", "off"])],
    },
    Task {
        id: "flush_dns",
        name: tr("Vider le cache DNS", "Flush the DNS cache"),
        detail: tr("Règle les sites qui ne chargent plus ou qui affichent une ancienne version.", "Fixes sites that no longer load or show an old version."),
        duration: tr("quelques secondes", "a few seconds"),
        needs_admin: false,
        steps: &[("ipconfig.exe", &["/flushdns"])],
    },
    Task {
        id: "restart_explorer",
        name: tr("Redémarrer l'Explorateur", "Restart Explorer"),
        detail: tr("Débloque la barre des tâches, le menu Démarrer ou le Bureau quand ils sont figés.", "Unfreezes the taskbar, Start menu or Desktop when they're stuck."),
        duration: tr("quelques secondes", "a few seconds"),
        needs_admin: false,
        steps: &[("taskkill.exe", &["/F", "/IM", "explorer.exe"]), ("explorer.exe", &[])],
    },
    Task {
        id: "refresh_icons",
        name: tr("Rafraîchir les icônes", "Refresh icons"),
        detail: tr("Corrige les icônes blanches ou mauvaises sur le Bureau et dans l'Explorateur.", "Fixes blank or wrong icons on the Desktop and in Explorer."),
        duration: tr("quelques secondes", "a few seconds"),
        needs_admin: false,
        steps: &[("ie4uinit.exe", &["-show"])],
    },
    Task {
        id: "reset_store",
        name: tr("Réparer le Microsoft Store", "Repair the Microsoft Store"),
        detail: tr("Vide le cache du Store quand les téléchargements ou les mises à jour d'applis bloquent.", "Clears the Store cache when app downloads or updates get stuck."),
        duration: tr("quelques secondes", "a few seconds"),
        needs_admin: false,
        steps: &[("wsreset.exe", &[])],
    },
    ]
}

#[derive(Serialize)]
pub struct TaskInfo {
    id: &'static str,
    name: &'static str,
    detail: &'static str,
    duration: &'static str,
    needs_admin: bool,
}

#[derive(Serialize)]
pub struct TaskResult {
    ok: bool,
    message: String,
}

pub fn list() -> Vec<TaskInfo> {
    tasks()
        .iter()
        .map(|t| TaskInfo { id: t.id, name: t.name, detail: t.detail, duration: t.duration, needs_admin: t.needs_admin })
        .collect()
}

fn expand(arg: &str) -> String {
    arg.replace("%SystemDrive%", &crate::garde::system_drive())
}

/// Texte d'une sortie d'outil. SFC écrit en UTF-16, les autres en 8 bits.
fn decode(output: &[u8], utf16: bool) -> String {
    if utf16 {
        let wide: Vec<u16> = output.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&wide)
    } else {
        String::from_utf8_lossy(output).into_owned()
    }
    .replace('\0', "")
}

/// Un octet nul en 2e position : c'est de l'UTF-16 (little-endian).
fn is_utf16(output: &[u8]) -> bool {
    output.len() >= 2 && output[1] == 0
}

/// Dernier pourcentage affiché par l'outil : « [==== 42.5% ====] » (DISM), « 67% complete » (SFC).
fn last_percent(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    for (i, _) in text.rmatch_indices('%') {
        let mut start = i;
        while start > 0 && bytes[start - 1] == b' ' {
            start -= 1;
        }
        let end = start;
        while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.' || bytes[start - 1] == b',') {
            start -= 1;
        }
        let number = text[start..end].replace(',', ".");
        if let Ok(value) = number.parse::<f32>() {
            if (0.0..=100.0).contains(&value) {
                return Some(value as u32);
            }
        }
    }
    None
}

/// Dernières lignes utiles de la sortie d'un outil.
fn tail(output: &[u8]) -> String {
    let text = decode(output, is_utf16(output));
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    lines[lines.len().saturating_sub(2)..].join(" ")
}

fn explorer_running() -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let running = sys.processes_by_exact_name(std::ffi::OsStr::new("explorer.exe")).next().is_some();
    running
}

/// Lance un programme en lisant sa sortie au fil de l'eau pour remonter son pourcentage.
/// Renvoie (code de sortie, sortie complète).
fn run_streaming(cmd: &mut Command, label: &str, progress: &dyn Fn(&str)) -> Result<(i32, Vec<u8>), String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("{label} : {e}"))?;
    let mut stdout = child.stdout.take().ok_or(tr("Sortie indisponible", "Output unavailable"))?;
    let mut output = vec![];
    let mut chunk = [0u8; 4096];
    let mut shown = None;
    loop {
        let n = stdout.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..n]);
        // Seule la fin de la sortie nous intéresse ; on garde une longueur paire pour l'UTF-16.
        let from = output.len().saturating_sub(2048) & !1;
        let percent = last_percent(&decode(&output[from..], is_utf16(&output)));
        if percent.is_some() && percent != shown {
            shown = percent;
            progress(&format!("{label} · {} %", percent.unwrap_or(0)));
        }
    }
    let mut errors = vec![];
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut errors);
    }
    let status = child.wait().map_err(|e| format!("{label} : {e}"))?;
    if !status.success() && !errors.is_empty() {
        output.extend_from_slice(b"\n");
        output.extend_from_slice(&errors);
    }
    Ok((status.code().unwrap_or(-1), output))
}

pub fn run(id: &str, progress: &dyn Fn(&str)) -> Result<TaskResult, String> {
    let tasks = tasks();
    let task = tasks.iter().find(|t| t.id == id).ok_or(tr("Tâche inconnue", "Unknown task"))?;
    let total = task.steps.len();
    for (i, (program, args)) in task.steps.iter().enumerate() {
        let name = program.trim_end_matches(".exe");
        let label = match (anglais(), total > 1) {
            (true, true) => format!("Step {} of {total} · {name}", i + 1),
            (true, false) => format!("{name} running"),
            (false, true) => format!("Étape {} sur {total} · {name}", i + 1),
            (false, false) => format!("{name} en cours"),
        };
        progress(&label);

        // Windows relance normalement l'Explorateur tout seul quand il s'arrête. On ne le lance
        // nous-mêmes que s'il n'est pas revenu, pour éviter d'ouvrir une fenêtre en trop.
        if *program == "explorer.exe" {
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if explorer_running() {
                    break;
                }
            }
            if !explorer_running() {
                Command::new(crate::garde::windows_program("explorer.exe")).spawn().map_err(|e| format!("explorer : {e}"))?;
            }
            continue;
        }

        // Chemin complet dans C:\Windows : jamais un programme du même nom posé ailleurs.
        let mut cmd = Command::new(crate::garde::windows_program(program));
        cmd.args(args.iter().map(|a| expand(a)));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let (code, output) = run_streaming(&mut cmd, &label, progress)?;
        // chkdsk : 1 = erreurs trouvées et corrigées, 2 = petit nettoyage fait. Les deux sont des réussites.
        let ok = code == 0 || (*program == "chkdsk.exe" && code <= 2) || (*program == "taskkill.exe");
        if !ok {
            let detail = tail(&output);
            return Ok(TaskResult {
                ok: false,
                message: match (detail.is_empty(), anglais()) {
                    (false, _) => detail,
                    (true, true) => format!("{name} failed (code {code})"),
                    (true, false) => format!("{name} a échoué (code {code})"),
                },
            });
        }
    }
    let message = match id {
        "repair_windows" => tr("Vérification terminée. Si des fichiers ont été réparés, redémarre ton PC.", "Check complete. If files were repaired, restart your PC."),
        "component_cleanup" => tr("Anciennes versions supprimées.", "Old versions removed."),
        "optimize_drive" => tr("Disque optimisé.", "Drive optimized."),
        "check_disk" => tr("Disque vérifié : aucun problème bloquant.", "Drive checked: no blocking issues."),
        "flush_dns" => tr("Cache DNS vidé.", "DNS cache flushed."),
        "restart_explorer" => tr("Explorateur redémarré.", "Explorer restarted."),
        "refresh_icons" => tr("Icônes rafraîchies.", "Icons refreshed."),
        "reset_store" => tr("Store réinitialisé : il va s'ouvrir tout seul.", "Store reset: it will open on its own."),
        "restore_point" => tr("Point de restauration créé. Windows n'en garde qu'un par 24 h : si un point récent existait, c'est lui qui sert.", "Restore point created. Windows keeps only one per 24 h: if a recent one existed, that's the one being used."),
        "hibernate_off" => tr("Veille prolongée désactivée, hiberfil.sys supprimé. Pour la réactiver : powercfg /h on.", "Hibernation turned off, hiberfil.sys deleted. To turn it back on: powercfg /h on."),
        _ => tr("Terminé.", "Done."),
    };
    Ok(TaskResult { ok: true, message: message.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_last_useful_lines_of_utf16_output() {
        let utf16: Vec<u8> = "Ligne 1\r\n\r\nPas de violation\r\nTerminé\r\n"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        assert_eq!(tail(&utf16), "Pas de violation Terminé");
    }

    #[test]
    fn reads_progress_percentages() {
        assert_eq!(last_percent("[=====     10.0%    ]\r[==========  42.5%  ]"), Some(42));
        assert_eq!(last_percent("Verification 67% complete."), Some(67));
        assert_eq!(last_percent("Progression : 100 %"), Some(100));
        assert_eq!(last_percent("aucun chiffre"), None);
        assert_eq!(last_percent("250% plus rapide"), None);
    }

    #[test]
    fn every_task_has_steps_and_a_unique_id() {
        let tasks = tasks();
        let mut ids: Vec<_> = tasks.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), tasks.len());
        assert!(tasks.iter().all(|t| !t.steps.is_empty()));
    }

    #[cfg(windows)]
    #[test]
    fn every_program_is_found_inside_windows() {
        for task in tasks() {
            for (program, _) in task.steps {
                let path = crate::garde::windows_program(program);
                assert!(path.is_file(), "{program} introuvable : {}", path.display());
            }
        }
    }
}
