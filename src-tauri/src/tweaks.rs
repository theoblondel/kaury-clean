//! Astuces : de petits réglages de Windows qui rendent le PC plus rapide ou plus calme.
//!
//! Chaque astuce n'écrit que des valeurs connues, fixées ici ; l'interface n'envoie que l'identifiant.
//! Avant la première écriture, la valeur d'origine est notée dans un fichier de sauvegarde : désactiver
//! l'astuce remet exactement ce qu'il y avait (ou le réglage de Windows si la sauvegarde manque).
//! Rien n'est supprimé, aucun service ni composant de Windows n'est coupé.

use serde::Serialize;

#[derive(Serialize)]
pub struct TweakInfo {
    id: &'static str,
    group: &'static str,
    name: &'static str,
    detail: &'static str,
    needs_admin: bool,
    /// "" : immédiat. "explorer" : après redémarrage de l'Explorateur. "signout" : à la prochaine session.
    after: &'static str,
    enabled: bool,
}

#[cfg(windows)]
mod imp {
    use super::TweakInfo;
    use crate::langue::tr;
    use std::collections::HashMap;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    use serde::{Deserialize, Serialize};
    use winreg::enums::*;
    use winreg::{RegKey, RegValue, HKEY};

    #[derive(Clone, Copy)]
    pub(super) enum Data {
        Dword(u32),
        Sz(&'static str),
    }

    pub(super) struct Val {
        pub hive: HKEY,
        pub path: &'static str,
        pub name: &'static str,
        pub on: Data,
        /// Réglage de Windows, si la sauvegarde manque. None : la valeur n'existe pas par défaut.
        pub default: Option<Data>,
        /// Clé créée par l'astuce, retirée entière quand on revient à « absente ».
        pub remove_key: Option<&'static str>,
    }

    pub(super) enum Action {
        Registry(&'static [Val]),
        /// Mode d'alimentation « Performances élevées » (powercfg).
        PowerPlan,
    }

    pub(super) struct Tweak {
        pub id: &'static str,
        pub group: &'static str,
        pub name: &'static str,
        pub detail: &'static str,
        pub needs_admin: bool,
        pub after: &'static str,
        pub win11_only: bool,
        pub action: Action,
    }

    const ADV: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
    const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
    const CLASSIC_MENU: &str = r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}";

    const fn v(hive: HKEY, path: &'static str, name: &'static str, on: Data, default: Option<Data>) -> Val {
        Val { hive, path, name, on, default, remove_key: None }
    }
    const fn cdm_off(name: &'static str) -> Val {
        v(HKEY_CURRENT_USER, CDM, name, Data::Dword(0), None)
    }

    const MENU_DELAY: &[Val] = &[v(HKEY_CURRENT_USER, r"Control Panel\Desktop", "MenuShowDelay", Data::Sz("100"), Some(Data::Sz("400")))];
    const ANIMATIONS: &[Val] = &[
        v(HKEY_CURRENT_USER, r"Control Panel\Desktop\WindowMetrics", "MinAnimate", Data::Sz("0"), Some(Data::Sz("1"))),
        v(HKEY_CURRENT_USER, ADV, "TaskbarAnimations", Data::Dword(0), Some(Data::Dword(1))),
    ];
    const TRANSPARENCY: &[Val] = &[v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency", Data::Dword(0), Some(Data::Dword(1)))];
    const THIS_PC: &[Val] = &[v(HKEY_CURRENT_USER, ADV, "LaunchTo", Data::Dword(1), None)];
    const START_WEB: &[Val] = &[v(HKEY_CURRENT_USER, r"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", Data::Dword(1), None)];
    const WIDGETS: &[Val] = &[v(HKEY_LOCAL_MACHINE, r"SOFTWARE\Policies\Microsoft\Dsh", "AllowNewsAndInterests", Data::Dword(0), None)];
    const GAME_DVR: &[Val] = &[
        v(HKEY_CURRENT_USER, r"System\GameConfigStore", "GameDVR_Enabled", Data::Dword(0), Some(Data::Dword(1))),
        v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", Data::Dword(0), None),
    ];
    const MOUSE: &[Val] = &[
        v(HKEY_CURRENT_USER, r"Control Panel\Mouse", "MouseSpeed", Data::Sz("0"), Some(Data::Sz("1"))),
        v(HKEY_CURRENT_USER, r"Control Panel\Mouse", "MouseThreshold1", Data::Sz("0"), Some(Data::Sz("6"))),
        v(HKEY_CURRENT_USER, r"Control Panel\Mouse", "MouseThreshold2", Data::Sz("0"), Some(Data::Sz("10"))),
    ];
    const SUGGESTIONS: &[Val] = &[
        cdm_off("SubscribedContent-338389Enabled"), // astuces et conseils en notification
        cdm_off("SubscribedContent-338393Enabled"), // contenu suggéré dans les Paramètres
        cdm_off("SubscribedContent-353694Enabled"),
        cdm_off("SubscribedContent-353696Enabled"),
        cdm_off("SubscribedContent-338388Enabled"), // suggestions dans le menu Démarrer (Windows 10)
        cdm_off("SystemPaneSuggestionsEnabled"),
        cdm_off("SoftLandingEnabled"),
        cdm_off("SubscribedContent-310093Enabled"), // « bienvenue » après les mises à jour
        v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement", "ScoobeSystemSettingEnabled", Data::Dword(0), None),
    ];
    const LOCK_SCREEN: &[Val] = &[cdm_off("RotatingLockScreenOverlayEnabled"), cdm_off("SubscribedContent-338387Enabled")];
    const START_RECO: &[Val] = &[
        v(HKEY_CURRENT_USER, ADV, "Start_IrisRecommendations", Data::Dword(0), None),
        v(HKEY_CURRENT_USER, ADV, "Start_AccountNotifications", Data::Dword(0), None),
    ];
    const END_TASK: &[Val] = &[v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\TaskbarDeveloperSettings", "TaskbarEndTask", Data::Dword(1), None)];
    const FILE_EXT: &[Val] = &[v(HKEY_CURRENT_USER, ADV, "HideFileExt", Data::Dword(0), Some(Data::Dword(1)))];
    const CLASSIC: &[Val] = &[Val {
        hive: HKEY_CURRENT_USER,
        path: r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32",
        name: "",
        on: Data::Sz(""),
        default: None,
        remove_key: Some(CLASSIC_MENU),
    }];
    const AD_ID: &[Val] = &[v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", Data::Dword(0), None)];
    const TAILORED: &[Val] = &[v(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Privacy", "TailoredExperiencesWithDiagnosticDataEnabled", Data::Dword(0), None)];

    pub(super) fn tweaks() -> Vec<Tweak> {
        let t = |id, group, name, detail, needs_admin, after, win11_only, action| Tweak { id, group, name, detail, needs_admin, after, win11_only, action };
        vec![
            // ---- Plus rapide
            t("power_high", "speed", tr("Mode d'alimentation « Performances élevées »", "\"High performance\" power plan"),
              tr("Le processeur ne ralentit plus pour économiser l'énergie : tout répond plus vite. Idéal sur un PC fixe ; sur un portable, la batterie dure moins longtemps.", "The processor no longer slows down to save power: everything responds faster. Ideal on a desktop PC; on a laptop, the battery runs out sooner."),
              false, "", false, Action::PowerPlan),
            t("menu_delay", "speed", tr("Menus sans délai", "Menus without delay"),
              tr("Les sous-menus (Ouvrir avec, Envoyer vers…) s'ouvrent tout de suite au lieu d'attendre 0,4 seconde.", "Submenus (Open with, Send to…) open right away instead of waiting 0.4 seconds."),
              false, "signout", false, Action::Registry(MENU_DELAY)),
            t("animations", "speed", tr("Couper les animations des fenêtres", "Turn off window animations"),
              tr("Les fenêtres s'ouvrent et se réduisent d'un coup, sans effet de zoom. Le PC paraît nettement plus vif, surtout s'il est ancien.", "Windows open and minimize instantly, with no zoom effect. The PC feels much snappier, especially an older one."),
              false, "signout", false, Action::Registry(ANIMATIONS)),
            t("transparency", "speed", tr("Couper la transparence", "Turn off transparency"),
              tr("La barre des tâches et le menu Démarrer deviennent opaques. Moins de travail pour la carte graphique, utile sur les petits PC.", "The taskbar and Start menu become opaque. Less work for the graphics card, useful on small PCs."),
              false, "", false, Action::Registry(TRANSPARENCY)),
            t("this_pc", "speed", tr("L'Explorateur s'ouvre sur « Ce PC »", "Explorer opens on \"This PC\""),
              tr("Au lieu de l'Accueil, qui charge tes fichiers récents et ceux du cloud : la fenêtre s'affiche plus vite.", "Instead of Home, which loads your recent and cloud files: the window shows up faster."),
              false, "", false, Action::Registry(THIS_PC)),
            t("start_web", "speed", tr("Recherche Windows sans Bing", "Windows search without Bing"),
              tr("La recherche du menu Démarrer ne cherche plus sur internet : elle trouve tes applis et fichiers plus vite, sans résultats web.", "Start menu search no longer looks on the internet: it finds your apps and files faster, without web results."),
              true, "explorer", false, Action::Registry(START_WEB)),
            t("widgets", "speed", tr("Désactiver les Widgets", "Turn off Widgets"),
              tr("Retire le panneau Actualités et météo et ses processus en arrière-plan, qui prennent souvent plusieurs centaines de Mo de mémoire. Les Paramètres afficheront « géré par ton organisation » pour ce réglage : c'est normal.", "Removes the news and weather panel and its background processes, which often use several hundred MB of memory. Settings will show \"managed by your organization\" for this option: that's normal."),
              true, "explorer", true, Action::Registry(WIDGETS)),
            // ---- Jeux
            t("game_dvr", "games", tr("Couper l'enregistrement en arrière-plan", "Turn off background recording"),
              tr("La Xbox Game Bar n'enregistre plus tes parties en continu : quelques images par seconde en plus dans les jeux. Tu peux toujours faire des captures avec Win + G.", "The Xbox Game Bar no longer records your games continuously: a few more frames per second in games. You can still take captures with Win + G."),
              false, "", false, Action::Registry(GAME_DVR)),
            t("mouse_accel", "games", tr("Souris sans accélération", "Mouse without acceleration"),
              tr("Désactive « Améliorer la précision du pointeur » : le curseur suit exactement ta main. Préféré pour les jeux et le graphisme.", "Turns off \"Enhance pointer precision\": the cursor follows your hand exactly. Preferred for games and design work."),
              false, "signout", false, Action::Registry(MOUSE)),
            // ---- Plus calme
            t("suggestions", "calm", tr("Plus de suggestions ni de pubs de Windows", "No more Windows suggestions or ads"),
              tr("Coupe les « astuces », les applis suggérées, les écrans de bienvenue après les mises à jour et les rappels pour « terminer la configuration ».", "Turns off \"tips\", suggested apps, welcome screens after updates and reminders to \"finish setting up\"."),
              false, "", false, Action::Registry(SUGGESTIONS)),
            t("lock_screen", "calm", tr("Écran de verrouillage sans anecdotes", "Lock screen without fun facts"),
              tr("Plus de textes « Le saviez-vous ? » ni de liens publicitaires sur l'écran de verrouillage.", "No more \"Did you know?\" texts or promotional links on the lock screen."),
              false, "", false, Action::Registry(LOCK_SCREEN)),
            t("start_reco", "calm", tr("Menu Démarrer sans recommandations", "Start menu without recommendations"),
              tr("Plus d'applis recommandées, d'astuces ni de rappels de compte Microsoft dans le menu Démarrer.", "No more recommended apps, tips or Microsoft account reminders in the Start menu."),
              false, "explorer", true, Action::Registry(START_RECO)),
            // ---- Pratique
            t("end_task", "handy", tr("« Fin de tâche » dans la barre des tâches", "\"End task\" in the taskbar"),
              tr("Clic droit sur une appli figée dans la barre des tâches › Fin de tâche. Plus besoin d'ouvrir le Gestionnaire des tâches.", "Right-click a frozen app in the taskbar › End task. No need to open Task Manager anymore."),
              false, "", true, Action::Registry(END_TASK)),
            t("file_ext", "handy", tr("Afficher les extensions des fichiers", "Show file extensions"),
              tr("Tu vois « facture.pdf.exe » au lieu de « facture.pdf » : le piège classique des virus ne marche plus.", "You see \"invoice.pdf.exe\" instead of \"invoice.pdf\": the classic virus trick no longer works."),
              false, "", false, Action::Registry(FILE_EXT)),
            t("classic_menu", "handy", tr("Menu clic droit complet", "Full right-click menu"),
              tr("Le clic droit affiche directement toutes les options, sans passer par « Afficher plus d'options ».", "Right-click shows every option right away, without going through \"Show more options\"."),
              false, "explorer", true, Action::Registry(CLASSIC)),
            // ---- Vie privée
            t("ad_id", "privacy", tr("Pas d'identifiant publicitaire", "No advertising ID"),
              tr("Les applis ne peuvent plus te suivre d'une appli à l'autre pour te montrer des pubs ciblées.", "Apps can no longer track you from one app to another to show targeted ads."),
              false, "", false, Action::Registry(AD_ID)),
            t("tailored", "privacy", tr("Pas d'expériences personnalisées", "No tailored experiences"),
              tr("Microsoft n'utilise plus tes données de diagnostic pour te proposer des astuces et des offres.", "Microsoft no longer uses your diagnostic data to show you tips and offers."),
              false, "", false, Action::Registry(TAILORED)),
        ]
    }

    // ---------- Registre ----------

    fn matches(key: &RegKey, val: &Val) -> bool {
        match val.on {
            Data::Dword(n) => key.get_value::<u32, _>(val.name).is_ok_and(|x| x == n),
            Data::Sz(s) => key.get_value::<String, _>(val.name).is_ok_and(|x| x == s),
        }
    }

    fn is_on(vals: &[Val]) -> bool {
        vals.iter().all(|val| RegKey::predef(val.hive).open_subkey_with_flags(val.path, KEY_READ).is_ok_and(|k| matches(&k, val)))
    }

    fn write(val: &Val, data: Data) -> Result<(), String> {
        let (key, _) = RegKey::predef(val.hive).create_subkey_with_flags(val.path, KEY_SET_VALUE).map_err(|_| admin_error())?;
        match data {
            Data::Dword(n) => key.set_value(val.name, &n),
            Data::Sz(s) => key.set_value(val.name, &s.to_string()),
        }
        .map_err(|_| admin_error())
    }

    fn admin_error() -> String {
        tr("Droits administrateur nécessaires pour ce réglage", "Admin rights are required for this setting").into()
    }

    // ---------- Sauvegarde des valeurs d'origine ----------

    #[derive(Serialize, Deserialize, Clone)]
    struct Saved {
        /// None : la valeur n'existait pas.
        bytes: Option<Vec<u8>>,
        vtype: u32,
    }

    type Backup = HashMap<String, Vec<Saved>>;

    /// Rangé dans le dossier de données de l'appli, à côté du journal de rangement.
    fn backup_file() -> PathBuf {
        dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("studio.kaury.clean").join("reglages-origine.json")
    }

    fn load() -> Backup {
        std::fs::read(backup_file()).ok().and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default()
    }

    fn store(b: &Backup) {
        let f = backup_file();
        if let Some(p) = f.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(f, serde_json::to_vec_pretty(b).unwrap_or_default());
    }

    fn vtype_of(t: &RegType) -> u32 {
        match t {
            REG_SZ => 1,
            REG_EXPAND_SZ => 2,
            REG_BINARY => 3,
            REG_DWORD => 4,
            REG_QWORD => 11,
            _ => 0,
        }
    }

    fn regtype(n: u32) -> RegType {
        match n {
            1 => REG_SZ,
            2 => REG_EXPAND_SZ,
            3 => REG_BINARY,
            11 => REG_QWORD,
            _ => REG_DWORD,
        }
    }

    fn snapshot(val: &Val) -> Saved {
        let raw = RegKey::predef(val.hive).open_subkey_with_flags(val.path, KEY_READ).ok().and_then(|k| k.get_raw_value(val.name).ok());
        match raw {
            Some(r) => Saved { vtype: vtype_of(&r.vtype), bytes: Some(r.bytes) },
            None => Saved { bytes: None, vtype: 0 },
        }
    }

    fn restore(val: &Val, saved: Option<&Saved>) -> Result<(), String> {
        let root = RegKey::predef(val.hive);
        match saved {
            Some(Saved { bytes: Some(bytes), vtype }) => {
                let (key, _) = root.create_subkey_with_flags(val.path, KEY_SET_VALUE).map_err(|_| admin_error())?;
                key.set_raw_value(val.name, &RegValue { bytes: bytes.clone(), vtype: regtype(*vtype) }).map_err(|_| admin_error())
            }
            Some(Saved { bytes: None, .. }) => remove(val),
            None => match val.default {
                Some(d) => write(val, d),
                None => remove(val),
            },
        }
    }

    /// Revient à « valeur absente » : supprime la valeur, ou la clé que l'astuce avait créée.
    fn remove(val: &Val) -> Result<(), String> {
        let root = RegKey::predef(val.hive);
        if let Some(k) = val.remove_key {
            return match root.delete_subkey_all(k) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(admin_error()),
            };
        }
        match root.open_subkey_with_flags(val.path, KEY_SET_VALUE) {
            Ok(key) => match key.delete_value(val.name) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(admin_error()),
            },
            Err(_) => Ok(()), // la clé n'existe pas : rien à retirer
        }
    }

    // ---------- Mode d'alimentation ----------

    const HIGH: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
    const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
    const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

    fn powercfg(args: &[&str]) -> Option<String> {
        let out = Command::new("powercfg.exe").args(args).creation_flags(0x0800_0000).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_lowercase())
    }

    /// Premier GUID trouvé dans une sortie de powercfg.
    fn guid(text: &str) -> Option<String> {
        text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
            .find(|w| w.len() == 36 && w.matches('-').count() == 4)
            .map(str::to_string)
    }

    fn power_available() -> bool {
        powercfg(&["/list"]).is_some_and(|l| l.contains(HIGH))
    }

    fn power_active() -> Option<String> {
        powercfg(&["/getactivescheme"]).and_then(|o| guid(&o))
    }

    fn set_power(on: bool, backup: &mut Backup) -> Result<(), String> {
        let fail = || tr("Windows a refusé de changer le mode d'alimentation", "Windows refused to change the power plan").to_string();
        if on {
            if !backup.contains_key("power_high") {
                if let Some(cur) = power_active() {
                    backup.insert("power_high".into(), vec![Saved { bytes: Some(cur.into_bytes()), vtype: 0 }]);
                }
            }
            powercfg(&["/setactive", HIGH]).ok_or_else(fail)?;
        } else {
            let previous = backup
                .get("power_high")
                .and_then(|s| s.first())
                .and_then(|s| s.bytes.clone())
                .and_then(|b| String::from_utf8(b).ok())
                .filter(|g| g.len() == 36 && g != HIGH && g != ULTIMATE)
                .unwrap_or_else(|| BALANCED.into());
            powercfg(&["/setactive", &previous]).ok_or_else(fail)?;
            backup.remove("power_high");
        }
        Ok(())
    }

    // ---------- Interface ----------

    fn is_win11() -> bool {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
            .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
            .ok()
            .and_then(|b| b.parse::<u32>().ok())
            .is_some_and(|b| b >= 22000)
    }

    pub fn list() -> Vec<TweakInfo> {
        let win11 = is_win11();
        tweaks()
            .into_iter()
            .filter(|t| win11 || !t.win11_only)
            .filter_map(|t| {
                let enabled = match &t.action {
                    Action::Registry(vals) => is_on(vals),
                    Action::PowerPlan => {
                        if !power_available() {
                            return None; // portables « Modern Standby » : le mode n'existe pas
                        }
                        power_active().is_some_and(|g| g == HIGH || g == ULTIMATE)
                    }
                };
                Some(TweakInfo { id: t.id, group: t.group, name: t.name, detail: t.detail, needs_admin: t.needs_admin, after: t.after, enabled })
            })
            .collect()
    }

    pub fn set(id: &str, on: bool) -> Result<(), String> {
        let all = tweaks();
        let t = all.iter().find(|t| t.id == id).ok_or(tr("Réglage inconnu", "Unknown setting"))?;
        if t.needs_admin && !crate::elevation::is_elevated() {
            return Err(admin_error());
        }
        let mut backup = load();
        let result = match &t.action {
            Action::PowerPlan => set_power(on, &mut backup),
            Action::Registry(vals) => {
                if on {
                    // La valeur d'origine n'est notée qu'une fois : réactiver ne l'écrase pas.
                    backup.entry(t.id.to_string()).or_insert_with(|| vals.iter().map(snapshot).collect());
                    store(&backup);
                    vals.iter().try_for_each(|val| write(val, val.on))
                } else {
                    let saved = backup.get(t.id).cloned().unwrap_or_default();
                    let r = vals.iter().enumerate().try_for_each(|(i, val)| restore(val, saved.get(i)));
                    if r.is_ok() {
                        backup.remove(t.id);
                    }
                    r
                }
            }
        };
        store(&backup);
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ids_are_unique_and_texts_filled() {
            let all = tweaks();
            let mut ids: Vec<_> = all.iter().map(|t| t.id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), all.len());
            assert!(all.iter().all(|t| !t.name.is_empty() && !t.detail.is_empty()));
            assert!(all.iter().all(|t| ["speed", "games", "calm", "handy", "privacy"].contains(&t.group)));
            assert!(all.iter().all(|t| ["", "explorer", "signout"].contains(&t.after)));
        }

        /// HKLM et les clés de stratégie ne s'écrivent qu'en administrateur : l'interface doit le savoir.
        #[test]
        fn protected_keys_require_admin() {
            for t in tweaks() {
                if let Action::Registry(vals) = t.action {
                    for val in vals {
                        let protected = val.hive == HKEY_LOCAL_MACHINE || val.path.contains("Policies");
                        assert!(!protected || t.needs_admin, "{} doit demander les droits administrateur", t.id);
                        assert!(val.hive == HKEY_LOCAL_MACHINE || val.hive == HKEY_CURRENT_USER);
                    }
                }
            }
        }

        /// Une clé retirée en bloc doit être celle que l'astuce crée, jamais une clé de Windows.
        #[test]
        fn removed_keys_are_our_own() {
            for t in tweaks() {
                if let Action::Registry(vals) = t.action {
                    for val in vals {
                        if let Some(k) = val.remove_key {
                            assert!(val.path.starts_with(k) && k.contains("{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}"));
                        }
                    }
                }
            }
        }

        #[test]
        fn reads_guid_from_powercfg() {
            let out = "guid du mode de gestion de l’alimentation : 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (haute performance)";
            assert_eq!(guid(out).as_deref(), Some(HIGH));
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::TweakInfo;
    pub fn list() -> Vec<TweakInfo> {
        Vec::new()
    }
    pub fn set(_id: &str, _on: bool) -> Result<(), String> {
        Err(crate::langue::tr("Pas encore disponible sur ce système", "Not available on this system yet").into())
    }
}

pub fn list() -> Vec<TweakInfo> {
    imp::list()
}

pub fn set(id: &str, on: bool) -> Result<(), String> {
    imp::set(id, on)
}
