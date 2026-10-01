//! Gros fichiers et doublons dans les dossiers perso (Bureau, Téléchargements, Documents, ...).
//! Ici on ne supprime jamais définitivement : tout part à la corbeille.

use crate::langue::{tr, anglais};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use crate::fsutil::modified_secs;
use crate::garde;

#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileEntry {
    bytes: u64,
    modified: u64,
    path: String,
    name: String,
    folder: String,
}

#[derive(Serialize)]
pub struct DuplicateGroup {
    bytes: u64,
    /// La copie la plus récente d'abord : c'est celle que l'interface propose de garder.
    files: Vec<FileEntry>,
}

#[derive(Serialize, Default)]
pub struct TrashReport {
    moved: u64,
    bytes: u64,
    errors: Vec<String>,
}

/// Passe à `true` quand tu cliques sur « Arrêter » : les recherches en cours s'arrêtent au fichier suivant.
pub static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn cancelled() -> bool {
    CANCEL.load(Ordering::Relaxed)
}

/// Nom affiché d'un dossier perso (Windows garde les noms anglais sur le disque).
fn display_name(root: &Path) -> String {
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match name.as_str() {
        "Desktop" => tr("Bureau", "Desktop").into(),
        "Downloads" => tr("Téléchargements", "Downloads").into(),
        "Documents" => "Documents".into(),
        "Videos" => tr("Vidéos", "Videos").into(),
        "Pictures" => tr("Images", "Pictures").into(),
        "Music" => tr("Musique", "Music").into(),
        _ => name,
    }
}

/// Dossiers perso analysés. Les dossiers système et les applis ne sont jamais parcourus.
pub fn user_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = [
        dirs::desktop_dir(),
        dirs::download_dir(),
        dirs::document_dir(),
        dirs::video_dir(),
        dirs::picture_dir(),
        dirs::audio_dir(),
    ]
    .into_iter()
    .flatten()
    // En administrateur, un dossier perso détourné hors de ton dossier personnel (vers C:\Windows...) est ignoré.
    .filter(|p| p.is_dir() && garde::user_dir_allowed(p))
    .collect();
    roots.sort();
    // Documents peut contenir d'autres dossiers de la liste (OneDrive) : on évite de compter deux fois.
    let mut out: Vec<PathBuf> = vec![];
    for r in roots {
        if !out.iter().any(|o| r.starts_with(o)) {
            out.push(r);
        }
    }
    out
}

fn is_hidden(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || name == "node_modules" || name == "$RECYCLE.BIN"
}

fn walk_files<'a>(
    roots: &'a [PathBuf],
    progress: &'a dyn Fn(&str),
) -> impl Iterator<Item = (PathBuf, std::fs::Metadata)> + 'a {
    roots.iter().flat_map(move |root| {
        progress(&display_name(root));
        WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_hidden(e))
            .take_while(|_| !cancelled())
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok().map(|m| (e.into_path(), m)))
            .filter(|(_, m)| !cloud_only(m))
    })
}

/// Fichier OneDrive (ou autre cloud) pas téléchargé sur le PC : il ne prend pas de place, et le lire
/// le téléchargerait. On l'ignore partout.
#[cfg(windows)]
fn cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const OFFLINE: u32 = 0x1000;
    const RECALL_ON_OPEN: u32 = 0x4_0000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
    meta.file_attributes() & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0
}

#[cfg(not(windows))]
fn cloud_only(_meta: &std::fs::Metadata) -> bool {
    false
}

fn entry(path: &Path, meta: &std::fs::Metadata) -> FileEntry {
    FileEntry {
        bytes: meta.len(),
        modified: modified_secs(meta),
        path: path.to_string_lossy().into(),
        name: path.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default(),
        folder: path.parent().map(|p| p.to_string_lossy().into()).unwrap_or_default(),
    }
}

/// Les `limit` plus gros fichiers d'au moins `min_bytes`, du plus lourd au plus léger.
pub fn large_files(roots: &[PathBuf], min_bytes: u64, limit: usize, progress: &dyn Fn(&str)) -> Vec<FileEntry> {
    let mut heap: BinaryHeap<Reverse<FileEntry>> = BinaryHeap::new();
    for (path, meta) in walk_files(roots, progress) {
        if meta.len() < min_bytes {
            continue;
        }
        heap.push(Reverse(entry(&path, &meta)));
        if heap.len() > limit {
            heap.pop();
        }
    }
    let mut out: Vec<FileEntry> = heap.into_iter().map(|Reverse(e)| e).collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

/// Les fichiers de Téléchargements pas modifiés depuis `min_days` jours, du plus lourd au plus léger.
pub fn old_downloads(downloads: &Path, min_days: u64, progress: &dyn Fn(&str)) -> Vec<FileEntry> {
    let limit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(min_days * 86_400))
        .unwrap_or(0);
    let roots = [downloads.to_path_buf()];
    let mut out: Vec<FileEntry> = walk_files(&roots, progress)
        .map(|(p, m)| entry(&p, &m))
        .filter(|e| e.modified > 0 && e.modified < limit)
        .collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out.truncate(300);
    out
}

fn hash_file(path: &Path, max_bytes: Option<u64>) -> io::Result<[u8; 32]> {
    let file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut reader: Box<dyn Read> = match max_bytes {
        Some(n) => Box::new(file.take(n)),
        None => Box::new(file),
    };
    io::copy(&mut reader, &mut hasher)?;
    Ok(*hasher.finalize().as_bytes())
}

/// Regroupe des chemins par empreinte, et ne garde que les groupes d'au moins deux fichiers.
type Found = (PathBuf, std::fs::Metadata);

fn group_by_hash(paths: Vec<Found>, max_bytes: Option<u64>, tick: &dyn Fn()) -> Vec<Vec<Found>> {
    let mut by_hash: HashMap<[u8; 32], Vec<Found>> = HashMap::new();
    for (p, m) in paths {
        if cancelled() {
            break;
        }
        tick();
        if let Ok(h) = hash_file(&p, max_bytes) {
            by_hash.entry(h).or_default().push((p, m));
        }
    }
    by_hash.into_values().filter(|g| g.len() > 1).collect()
}

/// Fichiers identiques au contenu près. On compare d'abord la taille, puis le début du fichier,
/// et seulement ensuite le fichier entier : c'est ce qui garde l'analyse rapide.
pub fn duplicates(roots: &[PathBuf], min_bytes: u64, progress: &dyn Fn(&str)) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<Found>> = HashMap::new();
    for (path, meta) in walk_files(roots, progress) {
        if meta.len() >= min_bytes {
            by_size.entry(meta.len()).or_default().push((path, meta));
        }
    }
    let candidates: Vec<Vec<Found>> = by_size.into_values().filter(|g| g.len() > 1).collect();
    let total: usize = candidates.iter().map(Vec::len).sum();
    let done = std::cell::Cell::new(0usize);
    let tick = || {
        done.set(done.get() + 1);
        if done.get().is_multiple_of(20) {
            let n = done.get().min(total);
            progress(&if anglais() { format!("{n} of {total} files compared") } else { format!("{n} fichiers comparés sur {total}") });
        }
    };
    let mut groups: Vec<DuplicateGroup> = candidates
        .into_iter()
        .flat_map(|g| group_by_hash(g, Some(64 * 1024), &tick))
        .flat_map(|g| group_by_hash(g, None, &|| {}))
        .map(|g| {
            let mut files: Vec<FileEntry> = g.iter().map(|(p, m)| entry(p, m)).collect();
            files.sort_by(|a, b| b.modified.cmp(&a.modified));
            DuplicateGroup { bytes: files[0].bytes, files }
        })
        .collect();
    // Les groupes qui font gagner le plus de place en premier.
    groups.sort_by_key(|g| Reverse(g.bytes * (g.files.len() as u64 - 1)));
    groups
}

/// Envoie des fichiers à la corbeille, seulement s'ils sont dans les dossiers perso analysés.
pub fn move_to_trash(roots: &[PathBuf], paths: &[String]) -> TrashReport {
    let mut report = TrashReport::default();
    for p in paths {
        let path = Path::new(p);
        let Some(real) = inside_roots(roots, path) else {
            report.errors.push(if anglais() { format!("{p}: file not found or outside the scanned folders") } else { format!("{p} : fichier introuvable ou hors des dossiers analysés") });
            continue;
        };
        let size = real.metadata().map(|m| m.len()).unwrap_or(0);
        // On jette le fichier vérifié, pas le chemin reçu de l'interface.
        match trash::delete(garde::plain(&real)) {
            Ok(()) => {
                report.moved += 1;
                report.bytes += size;
            }
            Err(e) => report.errors.push(format!("{p} : {e}")),
        }
    }
    report
}

/// Chemin réel d'un fichier, seulement s'il est dans les dossiers perso analysés.
fn inside_roots(roots: &[PathBuf], path: &Path) -> Option<PathBuf> {
    let roots: Vec<PathBuf> = roots.iter().filter_map(|r| r.canonicalize().ok()).collect();
    path.canonicalize().ok().filter(|c| c.is_file() && roots.iter().any(|r| c.starts_with(r)))
}

/// Ouvre l'Explorateur sur le dossier du fichier, avec le fichier sélectionné.
pub fn reveal(roots: &[PathBuf], path: &str) -> Result<(), String> {
    let path = garde::plain(&inside_roots(roots, Path::new(path)).ok_or(tr("Fichier introuvable", "File not found"))?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(garde::windows_program("explorer.exe"))
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    // macOS : le Finder s'ouvre sur le dossier, avec le fichier sélectionné.
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open").arg("-R").arg(&path).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = path;
        Err(tr("Disponible uniquement sur Windows", "Only available on Windows").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(path).unwrap().write_all(content).unwrap();
    }

    #[test]
    fn finds_largest_files_in_order() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("small.txt"), &[1; 10]);
        write(&dir.path().join("big.mov"), &[1; 3000]);
        write(&dir.path().join("sub/mid.zip"), &[1; 2000]);
        write(&dir.path().join(".hidden/huge.bin"), &[1; 9000]);
        let roots = vec![dir.path().to_path_buf()];
        let found = large_files(&roots, 1000, 10, &|_| {});
        let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["big.mov", "mid.zip"]);
        assert_eq!(large_files(&roots, 1000, 1, &|_| {}).len(), 1);
    }

    #[test]
    fn old_downloads_keeps_only_old_files() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("vieux.zip");
        write(&old, &[1; 500]);
        write(&dir.path().join("recent.zip"), &[1; 900]);
        let two_years_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(730 * 86_400);
        File::options().write(true).open(&old).unwrap().set_modified(two_years_ago).unwrap();
        let found = old_downloads(dir.path(), 90, &|_| {});
        let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["vieux.zip"]);
    }

    #[test]
    fn groups_identical_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let logo = vec![42u8; 200_000];
        let mut other = logo.clone();
        *other.last_mut().unwrap() = 0; // même taille, même début, fin différente
        write(&dir.path().join("Bureau/logo.ai"), &logo);
        write(&dir.path().join("Downloads/logo (1).ai"), &logo);
        write(&dir.path().join("Projets/logo_final.ai"), &logo);
        write(&dir.path().join("Projets/presque.ai"), &other);
        let groups = duplicates(&[dir.path().to_path_buf()], 1, &|_| {});
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 3);
        assert_eq!(groups[0].bytes, 200_000);
    }

    #[test]
    fn refuses_to_trash_outside_roots() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("system.dll");
        write(&target, b"x");
        let report = move_to_trash(&[root.path().to_path_buf()], &[target.to_string_lossy().into()]);
        assert_eq!(report.moved, 0);
        assert_eq!(report.errors.len(), 1);
        assert!(target.exists());
    }
}
