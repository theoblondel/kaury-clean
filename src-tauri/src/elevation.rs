//! Droits administrateur : savoir si on les a, et relancer l'appli avec.

#[cfg(windows)]
pub fn is_elevated() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

/// ShellExecute : ouvre un programme, un installeur, une adresse web ou un mailto comme un double-clic.
/// Avec le verbe « runas », Windows demande les droits administrateur.
#[cfg(windows)]
fn shell_execute(verb: &str, file: &str, params: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, file, params) = (wide(verb), wide(file), wide(params));
    let result = unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), SW_SHOWNORMAL)
    };
    // ShellExecute renvoie une valeur > 32 quand le lancement a réussi.
    if result as isize > 32 {
        Ok(())
    } else {
        Err("Lancement annulé".into())
    }
}

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
pub fn shell_open(file: &str, params: &str) -> Result<(), String> {
    shell_execute("open", file, params)
}

/// Relance l'appli en administrateur. Windows affiche sa fenêtre de confirmation (UAC) ;
/// si tu refuses, on renvoie une erreur et l'appli actuelle reste ouverte.
#[cfg(windows)]
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    shell_execute("runas", &exe.to_string_lossy(), "").map_err(|_| "Relance annulée".into())
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn relaunch_as_admin() -> Result<(), String> {
    Err("Disponible uniquement sur Windows".into())
}

#[cfg(not(windows))]
pub fn shell_open(_file: &str, _params: &str) -> Result<(), String> {
    Err("Disponible uniquement sur Windows".into())
}
