//! Gros fichiers et doublons dans les dossiers perso (Bureau, Téléchargements, Documents, ...).
//! Ici on ne supprime jamais définitivement : tout part à la corbeille.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use crate::fsutil::modified_secs;

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
    .filter(|p| p.is_dir())
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
        progress(&root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_hidden(e))
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok().map(|m| (e.into_path(), m)))
    })
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
            progress(&format!("{} fichiers comparés sur {total}", done.get().min(total)));
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
            report.errors.push(format!("{p} : fichier introuvable ou hors des dossiers analysés"));
            continue;
        };
        let size = real.metadata().map(|m| m.len()).unwrap_or(0);
        match trash::delete(path) {
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
    let path = Path::new(path);
    inside_roots(roots, path).ok_or("Fichier introuvable")?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Disponible uniquement sur Windows".into())
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
