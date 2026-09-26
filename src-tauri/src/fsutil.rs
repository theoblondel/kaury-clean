//! Outils fichiers partagés : mesurer un dossier et vider son contenu sans jamais toucher au dossier lui-même.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use walkdir::WalkDir;

/// Taille totale et nombre de fichiers d'un dossier (les liens et jonctions ne sont pas suivis).
/// Avec `min_age`, seuls les fichiers plus vieux que cette durée sont comptés.
pub fn dir_size(root: &Path, min_age: Duration) -> (u64, u64) {
    let now = SystemTime::now();
    let mut bytes = 0;
    let mut files = 0;
    for entry in WalkDir::new(root).min_depth(1).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if old_enough(&meta, now, min_age) {
                bytes += meta.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

/// Vide le contenu de `root` et garde le dossier. Les fichiers verrouillés (utilisés par une appli)
/// ou trop récents sont ignorés. Renvoie (octets libérés, fichiers supprimés, fichiers ignorés).
pub fn clean_dir(root: &Path, min_age: Duration) -> (u64, u64, u64) {
    let now = SystemTime::now();
    let mut freed = 0;
    let mut removed = 0;
    let mut skipped = 0;
    // contents_first : on voit les fichiers d'un dossier avant le dossier, donc il peut être supprimé une fois vide.
    for entry in WalkDir::new(root).min_depth(1).contents_first(true).into_iter().flatten() {
        let path = entry.path();
        let ft = entry.file_type();
        if ft.is_symlink() {
            // Un lien ou une jonction : on retire le lien, jamais ce vers quoi il pointe.
            let _ = fs::remove_file(path).or_else(|_| fs::remove_dir(path));
        } else if ft.is_dir() {
            // Échoue simplement si le dossier contient encore des fichiers ignorés.
            let _ = fs::remove_dir(path);
        } else {
            let Ok(meta) = entry.metadata() else { continue };
            if !old_enough(&meta, now, min_age) {
                skipped += 1;
                continue;
            }
            match fs::remove_file(path) {
                Ok(()) => {
                    freed += meta.len();
                    removed += 1;
                }
                Err(_) => skipped += 1,
            }
        }
    }
    (freed, removed, skipped)
}

fn old_enough(meta: &fs::Metadata, now: SystemTime, min_age: Duration) -> bool {
    if min_age.is_zero() {
        return true;
    }
    match meta.modified() {
        Ok(modified) => now.duration_since(modified).map(|age| age >= min_age).unwrap_or(false),
        Err(_) => false,
    }
}

pub fn modified_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn write(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(path).unwrap().write_all(&vec![7u8; size]).unwrap();
    }

    #[test]
    fn measures_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a.tmp"), 100);
        write(&dir.path().join("sub/b.tmp"), 50);
        assert_eq!(dir_size(dir.path(), Duration::ZERO), (150, 2));
    }

    #[test]
    fn cleans_contents_but_keeps_root() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a.tmp"), 100);
        write(&dir.path().join("sub/deep/b.tmp"), 50);
        let (freed, removed, skipped) = clean_dir(dir.path(), Duration::ZERO);
        assert_eq!((freed, removed, skipped), (150, 2, 0));
        assert!(dir.path().exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn keeps_recent_files_when_min_age_is_set() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("fresh.tmp"), 10);
        let (freed, _, skipped) = clean_dir(dir.path(), Duration::from_secs(3600));
        assert_eq!((freed, skipped), (0, 1));
        assert!(dir.path().join("fresh.tmp").exists());
        assert_eq!(dir_size(dir.path(), Duration::from_secs(3600)), (0, 0));
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_links_out_of_the_folder() {
        let outside = tempfile::tempdir().unwrap();
        write(&outside.path().join("precious.psd"), 10);
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        clean_dir(dir.path(), Duration::ZERO);
        assert!(outside.path().join("precious.psd").exists());
        assert!(!dir.path().join("link").exists());
    }
}
