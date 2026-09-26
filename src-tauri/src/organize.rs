//! Ranger un dossier en vrac (Téléchargements, Bureau) : chaque fichier va dans un sous-dossier
//! selon son type. Chaque rangement est noté pour pouvoir l'annuler.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

/// (dossier créé, extensions). Un fichier dont l'extension n'est dans aucune liste ne bouge pas.
const CATEGORIES: &[(&str, &[&str])] = &[
    ("Images", &["jpg", "jpeg", "png", "gif", "webp", "heic", "avif", "bmp", "tif", "tiff", "ico", "svg", "raw", "cr2", "cr3", "nef", "arw", "dng"]),
    ("Vidéos", &["mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "mts"]),
    ("Audio", &["mp3", "wav", "flac", "aac", "m4a", "ogg", "aif", "aiff"]),
    ("Design", &["psd", "ai", "indd", "idml", "eps", "fig", "sketch", "xd", "afdesign", "afphoto", "aep", "prproj", "blend", "c4d", "otf", "ttf", "woff", "woff2"]),
    ("Documents", &["pdf", "doc", "docx", "odt", "rtf", "txt", "md", "xls", "xlsx", "csv", "ods", "ppt", "pptx", "odp", "key", "pages", "numbers", "epub"]),
    ("Archives", &["zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz"]),
    ("Installeurs", &["exe", "msi", "msix", "appx", "iso", "dmg"]),
    ("Code", &["js", "ts", "py", "html", "css", "json", "rs", "php", "java", "c", "cpp", "h", "sh", "bat", "ps1", "xml", "yml", "yaml"]),
];

/// Fichiers jamais déplacés : raccourcis du Bureau et téléchargements en cours.
const KEEP: &[&str] = &["lnk", "url", "ini", "crdownload", "part", "partial", "download", "tmp"];

#[derive(Serialize)]
pub struct PlanGroup {
    category: String,
    count: u64,
    bytes: u64,
    examples: Vec<String>,
}

#[derive(Serialize, Default)]
pub struct OrganizeReport {
    moved: u64,
    errors: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Move {
    from: PathBuf,
    to: PathBuf,
}

fn category(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if KEEP.contains(&ext.as_str()) {
        return None;
    }
    CATEGORIES.iter().find(|(_, exts)| exts.contains(&ext.as_str())).map(|(name, _)| *name)
}

/// Les fichiers à ranger, directement dans `dir` (les sous-dossiers ne sont pas touchés).
fn candidates(dir: &Path) -> Vec<(PathBuf, &'static str, u64)> {
    let now = SystemTime::now();
    let Ok(read) = fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<_> = read
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() || e.file_name().to_string_lossy().starts_with('.') {
                return None;
            }
            // Un fichier modifié il y a moins de 2 minutes est peut-être encore en train d'arriver.
            let age = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).unwrap_or_default();
            if age < Duration::from_secs(120) {
                return None;
            }
            let cat = category(&e.path())?;
            Some((e.path(), cat, meta.len()))
        })
        .collect();
    out.sort();
    out
}

pub fn plan(dir: &Path) -> Vec<PlanGroup> {
    let files = candidates(dir);
    CATEGORIES
        .iter()
        .filter_map(|(cat, _)| {
            let mine: Vec<_> = files.iter().filter(|(_, c, _)| c == cat).collect();
            if mine.is_empty() {
                return None;
            }
            Some(PlanGroup {
                category: cat.to_string(),
                count: mine.len() as u64,
                bytes: mine.iter().map(|(_, _, b)| b).sum(),
                examples: mine
                    .iter()
                    .take(3)
                    .filter_map(|(p, _, _)| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect(),
            })
        })
        .collect()
}

/// Un nom libre dans `dir` : « photo.jpg », sinon « photo (2).jpg », « photo (3).jpg »...
fn free_name(dir: &Path, file: &Path) -> PathBuf {
    let target = dir.join(file.file_name().unwrap_or_default());
    if !target.exists() {
        return target;
    }
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = file.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap()
}

pub fn apply(dir: &Path, undo_log: &Path) -> OrganizeReport {
    let mut report = OrganizeReport::default();
    let mut moves = vec![];
    for (path, cat, _) in candidates(dir) {
        let dest_dir = dir.join(cat);
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            report.errors.push(format!("{cat} : {e}"));
            continue;
        }
        let to = free_name(&dest_dir, &path);
        match fs::rename(&path, &to) {
            Ok(()) => {
                report.moved += 1;
                moves.push(Move { from: path, to });
            }
            Err(e) => report.errors.push(format!("{} : {e}", path.display())),
        }
    }
    if !moves.is_empty() {
        if let Some(parent) = undo_log.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(undo_log, serde_json::to_vec(&moves).unwrap_or_default());
    }
    report
}

pub fn can_undo(undo_log: &Path) -> bool {
    undo_log.is_file()
}

/// Un déplacement noté ressemble-t-il vraiment à un rangement fait par l'appli ?
/// `dossier/Catégorie/fichier` ↔ `dossier/fichier`, avec `dossier` parmi ceux qu'on sait ranger.
/// Le journal est un fichier modifiable : sans ce contrôle, un programme pourrait y écrire de faux
/// chemins et faire déplacer des fichiers de Windows à l'annulation.
fn legit(m: &Move, dirs: &[PathBuf]) -> bool {
    let (Some(home), Some(cat_dir), Some(name)) = (m.from.parent(), m.to.parent(), m.from.file_name()) else {
        return false;
    };
    let is_category = cat_dir.file_name().and_then(|n| n.to_str()).is_some_and(|n| CATEGORIES.iter().any(|(c, _)| *c == n));
    let plain_name = std::path::Path::new(name).components().count() == 1;
    is_category && plain_name && cat_dir.parent() == Some(home) && dirs.iter().any(|d| d.as_path() == home)
}

/// Remet chaque fichier du dernier rangement à sa place, puis retire les dossiers de rangement vides.
/// `dirs` : les seuls dossiers où un rangement a pu avoir lieu.
pub fn undo(undo_log: &Path, dirs: &[PathBuf]) -> OrganizeReport {
    let mut report = OrganizeReport::default();
    let Ok(data) = fs::read(undo_log) else {
        report.errors.push("Aucun rangement à annuler".into());
        return report;
    };
    let moves: Vec<Move> = serde_json::from_slice::<Vec<Move>>(&data).unwrap_or_default().into_iter().filter(|m| legit(m, dirs)).collect();
    for m in moves.iter().rev() {
        if m.from.exists() {
            report.errors.push(format!("{} existe déjà, fichier laissé dans le dossier rangé", m.from.display()));
            continue;
        }
        match fs::rename(&m.to, &m.from) {
            Ok(()) => report.moved += 1,
            Err(e) => report.errors.push(format!("{} : {e}", m.to.display())),
        }
    }
    for m in &moves {
        if let Some(parent) = m.to.parent() {
            let _ = fs::remove_dir(parent); // échoue simplement s'il reste des fichiers
        }
    }
    let _ = fs::remove_file(undo_log);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn old_file(path: &Path) {
        File::create(path).unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        File::options().write(true).open(path).unwrap().set_modified(old).unwrap();
    }

    #[test]
    fn sorts_by_type_and_undoes() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for f in ["photo.JPG", "brief.pdf", "logo.ai", "setup.exe", "notes.xyz", "raccourci.lnk"] {
            old_file(&d.join(f));
        }
        File::create(d.join("en-cours.pdf")).unwrap(); // trop récent

        let cats: Vec<String> = plan(d).into_iter().map(|g| g.category).collect();
        assert_eq!(cats, ["Images", "Design", "Documents", "Installeurs"]);

        let log = d.join("undo.json");
        let report = apply(d, &log);
        assert_eq!(report.moved, 4);
        assert!(d.join("Images/photo.JPG").exists());
        assert!(d.join("Design/logo.ai").exists());
        assert!(d.join("notes.xyz").exists());
        assert!(d.join("raccourci.lnk").exists());
        assert!(d.join("en-cours.pdf").exists());

        let back = undo(&log, &[d.to_path_buf()]);
        assert_eq!(back.moved, 4);
        assert!(d.join("photo.JPG").exists());
        assert!(!d.join("Images").exists());
        assert!(!can_undo(&log));
    }

    #[test]
    fn never_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        fs::create_dir(d.join("Images")).unwrap();
        fs::write(d.join("Images/photo.png"), b"ancienne").unwrap();
        fs::write(d.join("photo.png"), b"nouvelle").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        File::options().write(true).open(d.join("photo.png")).unwrap().set_modified(old).unwrap();
        apply(d, &d.join("undo.json"));
        assert_eq!(fs::read(d.join("Images/photo.png")).unwrap(), b"ancienne");
        assert_eq!(fs::read(d.join("Images/photo (2).png")).unwrap(), b"nouvelle");
    }

    #[test]
    fn undo_ignores_forged_moves() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let d = dir.path();
        let precious = outside.path().join("systeme.dll");
        fs::write(&precious, b"x").unwrap();
        let forged = vec![
            Move { from: d.join("vole.dll"), to: precious.clone() },
            Move { from: d.join("Images/../../x.dll"), to: d.join("Images/x.dll") },
        ];
        let log = d.join("undo.json");
        fs::write(&log, serde_json::to_vec(&forged).unwrap()).unwrap();
        let report = undo(&log, &[d.to_path_buf()]);
        assert_eq!(report.moved, 0);
        assert!(precious.exists());
        assert!(!d.join("vole.dll").exists());
    }
}
