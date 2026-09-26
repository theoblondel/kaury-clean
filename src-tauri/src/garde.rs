//! Garde-fous : Kaury Clean tourne souvent en administrateur, il ne doit jamais devenir l'outil
//! d'un autre programme. Tout ce qui efface, déplace ou lance quelque chose passe par ici.
//!
//! Trois règles :
//! - on efface un fichier par son vrai emplacement, vérifié au moment même de l'effacer : un lien ou
//!   une jonction glissé dans un dossier ne peut pas rediriger l'effacement vers Windows ;
//! - en administrateur, les dossiers « de ton compte » ne sont acceptés que s'ils sont vraiment dans
//!   ton dossier personnel : une variable TEMP ou un dossier Documents détourné vers C:\Windows est ignoré ;
//! - les dossiers et programmes de Windows se trouvent par l'API de Windows, jamais par une variable
//!   d'environnement qu'un programme pourrait modifier.

use std::fs;
use std::path::{Path, PathBuf};

/// Chemin comparable : sans le préfixe « \\?\ », en minuscules, sans barre finale.
fn key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('/', "\\");
    let s = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    };
    s.trim_end_matches('\\').to_lowercase()
}

/// `path` est-il dans `root` (strictement en dessous) ? Les deux chemins doivent être réels (canonicalisés).
pub fn is_inside(path: &Path, root: &Path) -> bool {
    let (p, r) = (key(path), key(root));
    !r.is_empty() && p.len() > r.len() + 1 && p.starts_with(&r) && p[r.len()..].starts_with('\\')
}

/// Chemin réel lisible par l'Explorateur et la corbeille : « \\?\C:\x » devient « C:\x ».
pub fn plain(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Chemin réel d'un dossier à vider, seulement si aucun lien ni jonction ne se trouve sur le chemin.
/// Sinon le dossier mènerait ailleurs que prévu : on n'y touche pas.
pub fn safe_root(root: &Path) -> Option<PathBuf> {
    let real = fs::canonicalize(root).ok()?;
    for part in root.ancestors().filter(|a| a.parent().is_some()) {
        if fs::symlink_metadata(part).ok()?.file_type().is_symlink() {
            return None;
        }
    }
    real.is_dir().then_some(real)
}

#[cfg(windows)]
mod imp {
    use super::{is_inside, key};
    use std::ffi::c_void;
    use std::fs::OpenOptions;
    use std::io;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};

    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    const FILE_SHARE_ALL: u32 = 0x1 | 0x2 | 0x4;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_DISPOSITION_INFO: i32 = 4;
    /// Pseudo-handle du jeton du processus courant (GetCurrentProcessToken).
    const CURRENT_PROCESS_TOKEN: *mut c_void = -4isize as *mut c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFinalPathNameByHandleW(file: *mut c_void, path: *mut u16, len: u32, flags: u32) -> u32;
        fn SetFileInformationByHandle(file: *mut c_void, class: i32, info: *const c_void, size: u32) -> i32;
        fn GetSystemWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    }

    #[link(name = "userenv")]
    extern "system" {
        fn GetUserProfileDirectoryW(token: *mut c_void, dir: *mut u16, size: *mut u32) -> i32;
    }

    fn wide_to_path(buf: &[u16]) -> PathBuf {
        PathBuf::from(String::from_utf16_lossy(buf))
    }

    fn final_path(handle: *mut c_void) -> io::Result<PathBuf> {
        let mut buf = vec![0u16; 512];
        loop {
            let n = unsafe { GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), buf.len() as u32, 0) } as usize;
            if n == 0 {
                return Err(io::Error::last_os_error());
            }
            if n < buf.len() {
                return Ok(wide_to_path(&buf[..n]));
            }
            buf.resize(n + 1, 0);
        }
    }

    /// Efface `path` (fichier, dossier vide, lien ou jonction) seulement si son vrai emplacement,
    /// lu sur le fichier ouvert, est dans `root`. L'effacement se fait sur ce même fichier ouvert :
    /// rien ne peut être échangé entre la vérification et l'effacement.
    pub fn remove_inside(path: &Path, root: &Path) -> io::Result<()> {
        let file = OpenOptions::new()
            .access_mode(DELETE | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_ALL)
            // Ouvre le lien lui-même, jamais ce vers quoi il pointe ; accepte aussi les dossiers.
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let handle = file.as_raw_handle() as *mut c_void;
        if !is_inside(&final_path(handle)?, root) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "hors du dossier à nettoyer"));
        }
        let delete: u8 = 1; // FILE_DISPOSITION_INFO { DeleteFile: TRUE }
        let ok = unsafe { SetFileInformationByHandle(handle, FILE_DISPOSITION_INFO, (&delete as *const u8).cast(), 1) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(()) // l'effacement a lieu à la fermeture du fichier, ici
    }

    /// Le dossier Windows, donné par Windows lui-même.
    pub fn windows_dir() -> PathBuf {
        let mut buf = [0u16; 260];
        let n = unsafe { GetSystemWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n == 0 || n >= buf.len() {
            return PathBuf::from(r"C:\Windows");
        }
        wide_to_path(&buf[..n])
    }

    /// Ton dossier personnel (C:\Users\toi), tel que Windows l'a enregistré pour ton compte.
    pub fn profile_dir() -> Option<PathBuf> {
        let mut size = 0u32;
        unsafe { GetUserProfileDirectoryW(CURRENT_PROCESS_TOKEN, std::ptr::null_mut(), &mut size) };
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u16; size as usize];
        if unsafe { GetUserProfileDirectoryW(CURRENT_PROCESS_TOKEN, buf.as_mut_ptr(), &mut size) } == 0 {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let dir = wide_to_path(&buf[..end]);
        // Un profil à la racine d'un disque ne protégerait rien.
        (key(&dir).matches('\\').count() >= 1).then_some(dir)
    }

    pub fn elevated() -> bool {
        crate::elevation::is_elevated()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::is_inside;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    pub fn remove_inside(path: &Path, root: &Path) -> io::Result<()> {
        let parent = path.parent().and_then(|p| fs::canonicalize(p).ok()).unwrap_or_default();
        let real = parent.join(path.file_name().unwrap_or_default());
        if !is_inside(&real, root) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "hors du dossier à nettoyer"));
        }
        let meta = fs::symlink_metadata(path)?;
        if meta.is_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
    }

    pub fn windows_dir() -> PathBuf {
        PathBuf::from("/nonexistent-windows")
    }

    pub fn profile_dir() -> Option<PathBuf> {
        dirs::home_dir()
    }

    pub fn elevated() -> bool {
        false
    }
}

pub use imp::{profile_dir, remove_inside, windows_dir};

/// Un programme de Windows par son chemin complet : un faux « taskkill.exe » posé ailleurs
/// sur le PC ne peut jamais être lancé à sa place.
pub fn windows_program(name: &str) -> PathBuf {
    let windir = windows_dir();
    match name.to_ascii_lowercase().as_str() {
        "explorer.exe" => windir.join(name),
        "powershell.exe" => windir.join(r"System32\WindowsPowerShell\v1.0").join(name),
        _ => windir.join("System32").join(name),
    }
}

/// Le disque de Windows, « C: » en général.
pub fn system_drive() -> String {
    let windir = windows_dir();
    let s = windir.to_string_lossy();
    match s.as_bytes().get(1) {
        Some(b':') => s[..2].to_string(),
        _ => "C:".into(),
    }
}

/// Un dossier « de ton compte » peut-il être touché ? Toujours sans les droits administrateur
/// (l'appli ne peut alors rien faire que tu ne puisses faire toi-même). En administrateur,
/// seulement s'il est vraiment dans ton dossier personnel.
pub fn user_dir_allowed(dir: &Path) -> bool {
    if !imp::elevated() {
        return true;
    }
    let (Some(profile), Ok(real)) = (profile_dir().and_then(|p| fs::canonicalize(p).ok()), fs::canonicalize(dir)) else {
        return false;
    };
    is_inside(&real, &profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_real_paths_without_prefix_or_case() {
        assert!(is_inside(Path::new(r"\\?\C:\Windows\Temp\a.tmp"), Path::new(r"C:\windows\temp")));
        assert!(is_inside(Path::new(r"C:\Windows\Temp\sub\a.tmp"), Path::new(r"\\?\C:\Windows\Temp\")));
        assert!(!is_inside(Path::new(r"C:\Windows\Temp"), Path::new(r"C:\Windows\Temp")));
        assert!(!is_inside(Path::new(r"C:\Windows\Temporary\a"), Path::new(r"C:\Windows\Temp")));
        assert!(!is_inside(Path::new(r"C:\Windows\System32\kernel32.dll"), Path::new(r"C:\Windows\Temp")));
        assert!(!is_inside(Path::new(r"C:\x"), Path::new("")));
    }

    #[test]
    fn plain_paths_for_explorer() {
        assert_eq!(plain(Path::new(r"\\?\C:\Users\a\b.txt")), PathBuf::from(r"C:\Users\a\b.txt"));
        assert_eq!(plain(Path::new(r"\\?\UNC\srv\share\b.txt")), PathBuf::from(r"\\?\UNC\srv\share\b.txt"));
    }

    #[cfg(windows)]
    #[test]
    fn finds_windows_programs_by_full_path() {
        let taskkill = windows_program("taskkill.exe");
        assert!(taskkill.is_absolute() && taskkill.ends_with(r"System32\taskkill.exe"));
        assert!(taskkill.is_file());
        assert!(windows_program("explorer.exe").is_file());
        assert!(windows_program("powershell.exe").is_file());
        assert!(system_drive().ends_with(':'));
        assert!(profile_dir().is_some_and(|p| p.is_dir()));
    }

    #[test]
    fn refuses_to_remove_outside_the_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let precious = outside.path().join("precious.psd");
        fs::write(&precious, b"x").unwrap();
        let real_root = fs::canonicalize(root.path()).unwrap();
        assert!(remove_inside(&precious, &real_root).is_err());
        assert!(precious.exists());

        let junk = root.path().join("junk.tmp");
        fs::write(&junk, b"x").unwrap();
        remove_inside(&junk, &real_root).unwrap();
        assert!(!junk.exists());
    }
}
