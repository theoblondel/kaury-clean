//! Maintenance et réparation : les outils officiels de Windows, lancés pour toi avec les bonnes options.
//! Chaque commande est fixée ici ; l'interface n'envoie que l'identifiant de la tâche.

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

const TASKS: &[Task] = &[
    Task {
        id: "repair_windows",
        name: "Réparer Windows",
        detail: "Vérifie et répare les fichiers système abîmés (DISM puis SFC). À faire si Windows plante, affiche des erreurs bizarres ou si des applis ne s'ouvrent plus. Il faut Internet.",
        duration: "15 à 30 min",
        needs_admin: true,
        steps: &[
            ("DISM.exe", &["/Online", "/Cleanup-Image", "/RestoreHealth"]),
            ("sfc.exe", &["/scannow"]),
        ],
    },
    Task {
        id: "component_cleanup",
        name: "Supprimer les anciennes versions de Windows",
        detail: "Retire les composants remplacés par les mises à jour. Libère souvent plusieurs Go.",
        duration: "5 à 15 min",
        needs_admin: true,
        steps: &[("DISM.exe", &["/Online", "/Cleanup-Image", "/StartComponentCleanup"])],
    },
    Task {
        id: "optimize_drive",
        name: "Optimiser le disque",
        detail: "Envoie TRIM à un SSD ou défragmente un disque dur, selon ton matériel. Garde le disque rapide.",
        duration: "1 à 10 min",
        needs_admin: true,
        steps: &[("defrag.exe", &["%SystemDrive%", "/O"])],
    },
    Task {
        id: "check_disk",
        name: "Vérifier le disque",
        detail: "Cherche les erreurs du système de fichiers sans redémarrer.",
        duration: "2 à 10 min",
        needs_admin: true,
        steps: &[("chkdsk.exe", &["%SystemDrive%", "/scan"])],
    },
    Task {
        id: "flush_dns",
        name: "Vider le cache DNS",
        detail: "Règle les sites qui ne chargent plus ou qui affichent une ancienne version.",
        duration: "quelques secondes",
        needs_admin: false,
        steps: &[("ipconfig.exe", &["/flushdns"])],
    },
    Task {
        id: "restart_explorer",
        name: "Redémarrer l'Explorateur",
        detail: "Débloque la barre des tâches, le menu Démarrer ou le Bureau quand ils sont figés.",
        duration: "quelques secondes",
        needs_admin: false,
        steps: &[("taskkill.exe", &["/F", "/IM", "explorer.exe"]), ("explorer.exe", &[])],
    },
    Task {
        id: "refresh_icons",
        name: "Rafraîchir les icônes",
        detail: "Corrige les icônes blanches ou mauvaises sur le Bureau et dans l'Explorateur.",
        duration: "quelques secondes",
        needs_admin: false,
        steps: &[("ie4uinit.exe", &["-show"])],
    },
    Task {
        id: "reset_store",
        name: "Réparer le Microsoft Store",
        detail: "Vide le cache du Store quand les téléchargements ou les mises à jour d'applis bloquent.",
        duration: "quelques secondes",
        needs_admin: false,
        steps: &[("wsreset.exe", &[])],
    },
];

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
    TASKS
        .iter()
        .map(|t| TaskInfo { id: t.id, name: t.name, detail: t.detail, duration: t.duration, needs_admin: t.needs_admin })
        .collect()
}

fn expand(arg: &str) -> String {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    arg.replace("%SystemDrive%", &drive)
}

/// Dernières lignes utiles de la sortie d'un outil.
fn tail(output: &[u8]) -> String {
    // Un octet nul en 2e position : c'est de l'UTF-16 (little-endian), comme la sortie de SFC.
    let text = if output.len() >= 2 && output[1] == 0 {
        let wide: Vec<u16> = output.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&wide)
    } else {
        String::from_utf8_lossy(output).into_owned()
    }
    .replace('\0', "");
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    lines[lines.len().saturating_sub(2)..].join(" ")
}

pub fn run(id: &str, progress: &dyn Fn(&str)) -> Result<TaskResult, String> {
    let task = TASKS.iter().find(|t| t.id == id).ok_or("Tâche inconnue")?;
    let total = task.steps.len();
    for (i, (program, args)) in task.steps.iter().enumerate() {
        progress(&if total > 1 { format!("Étape {} sur {total} · {program}", i + 1) } else { format!("{program} en cours") });
        let mut cmd = Command::new(program);
        cmd.args(args.iter().map(|a| expand(a)));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        // L'Explorateur relancé doit continuer à vivre après nous : on ne l'attend pas.
        if *program == "explorer.exe" {
            cmd.spawn().map_err(|e| format!("{program} : {e}"))?;
            continue;
        }
        let out = cmd.output().map_err(|e| format!("{program} : {e}"))?;
        let code = out.status.code().unwrap_or(-1);
        // chkdsk : 1 = erreurs trouvées et corrigées, 2 = petit nettoyage fait. Les deux sont des réussites.
        let ok = code == 0 || (*program == "chkdsk.exe" && code <= 2) || (*program == "taskkill.exe");
        if !ok {
            let detail = tail(&out.stdout);
            return Ok(TaskResult {
                ok: false,
                message: if detail.is_empty() { format!("{program} a échoué (code {code})") } else { detail },
            });
        }
    }
    let message = match id {
        "repair_windows" => "Vérification terminée. Si des fichiers ont été réparés, redémarre ton PC.",
        "component_cleanup" => "Anciennes versions supprimées.",
        "optimize_drive" => "Disque optimisé.",
        "check_disk" => "Disque vérifié : aucun problème bloquant.",
        "flush_dns" => "Cache DNS vidé.",
        "restart_explorer" => "Explorateur redémarré.",
        "refresh_icons" => "Icônes rafraîchies.",
        "reset_store" => "Store réinitialisé : il va s'ouvrir tout seul.",
        _ => "Terminé.",
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
    fn every_task_has_steps_and_a_unique_id() {
        let mut ids: Vec<_> = TASKS.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), TASKS.len());
        assert!(TASKS.iter().all(|t| !t.steps.is_empty()));
    }
}
