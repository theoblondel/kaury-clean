//! Droits administrateur : savoir si on les a, et relancer l'appli avec.

#[cfg(windows)]
pub fn is_elevated() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

/// Relance l'appli en administrateur. Windows affiche sa fenêtre de confirmation (UAC) ;
/// si tu refuses, on renvoie une erreur et l'appli actuelle reste ouverte.
#[cfg(windows)]
pub fn relaunch_as_admin() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let result = unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL)
    };
    // ShellExecute renvoie une valeur > 32 quand le lancement a réussi.
    if result as isize > 32 {
        Ok(())
    } else {
        Err("Relance annulée".into())
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn relaunch_as_admin() -> Result<(), String> {
    Err("Disponible uniquement sur Windows".into())
}
