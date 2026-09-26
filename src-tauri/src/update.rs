//! Mises à jour : on regarde la dernière release publiée sur GitHub, et si elle est plus récente,
//! on télécharge son installeur puis on le lance. L'installeur remplace l'ancienne version en place.

use std::io::{Read, Write};
use std::path::PathBuf;

use serde::Serialize;

const REPO: &str = "theoblondel/kaury-clean";
/// Seuls les installeurs publiés dans les releases de ce repo sont acceptés.
const DOWNLOAD_PREFIX: &str = "https://github.com/theoblondel/kaury-clean/releases/download/";

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub available: bool,
    pub current: String,
    pub latest: String,
    pub notes: String,
    pub size: u64,
    pub page: String,
    #[serde(skip)]
    pub download_url: String,
    #[serde(skip)]
    pub file_name: String,
}

/// « v0.10.2 » → (0, 10, 2). Les morceaux illisibles comptent pour 0.
pub fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

/// Connexion chiffrée avec le TLS de Windows : il respecte les certificats et le proxy du PC.
fn agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20));
    if let Ok(tls) = native_tls::TlsConnector::new() {
        builder = builder.tls_connector(std::sync::Arc::new(tls));
    }
    builder.build()
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let release: serde_json::Value = agent()
        .get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .set("User-Agent", "KauryClean")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "Aucune version publiée pour l'instant".to_string(),
            _ => "Impossible de joindre GitHub. Vérifie ta connexion.".to_string(),
        })?
        .into_json()
        .map_err(|_| "Réponse de GitHub illisible".to_string())?;

    let latest = release["tag_name"].as_str().unwrap_or("").to_string();
    let asset = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with("-setup.exe")));
    let download_url = asset.and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string();

    Ok(UpdateInfo {
        available: is_newer(&latest, current) && download_url.starts_with(DOWNLOAD_PREFIX),
        current: current.to_string(),
        latest: latest.trim_start_matches('v').to_string(),
        notes: release["body"].as_str().unwrap_or("").to_string(),
        size: asset.and_then(|a| a["size"].as_u64()).unwrap_or(0),
        page: release["html_url"].as_str().unwrap_or("").to_string(),
        file_name: asset.and_then(|a| a["name"].as_str()).unwrap_or("kaury-clean-setup.exe").to_string(),
        download_url,
    })
}

/// Télécharge l'installeur de la dernière version dans le dossier temporaire.
/// `progress` reçoit le pourcentage téléchargé.
pub fn download(current: &str, progress: &dyn Fn(u32)) -> Result<PathBuf, String> {
    // On redemande la release ici : l'adresse ne vient jamais de l'interface.
    let info = check(current)?;
    if !info.available {
        return Err("Tu as déjà la dernière version".into());
    }
    let response = agent()
        .get(&info.download_url)
        .set("User-Agent", "KauryClean")
        .call()
        .map_err(|_| "Le téléchargement a échoué".to_string())?;
    let total: u64 = response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(info.size);

    let name: String = info.file_name.chars().filter(|c| c.is_ascii_alphanumeric() || "._- ".contains(*c)).collect();
    let path = std::env::temp_dir().join(name);
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 64 * 1024];
    let mut done = 0u64;
    let mut shown = u32::MAX;
    loop {
        let n = reader.read(&mut buffer).map_err(|_| "Le téléchargement a été coupé".to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if total > 0 {
            let pct = (done * 100 / total) as u32;
            if pct != shown {
                shown = pct;
                progress(pct);
            }
        }
    }
    if total > 0 && done != total {
        return Err("Le téléchargement est incomplet".into());
    }
    Ok(path)
}

/// Lance l'installeur téléchargé. Windows demande l'autorisation administrateur, puis
/// l'installeur remplace l'ancienne version.
#[cfg(windows)]
pub fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    crate::elevation::shell_open(&path.to_string_lossy(), "")
}

#[cfg(not(windows))]
pub fn launch_installer(_path: &std::path::Path) -> Result<(), String> {
    Err("Disponible uniquement sur Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("v0.6.0", "0.5.0"));
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("v0.5.0", "0.5.0"));
        assert!(!is_newer("v0.4.9", "0.5.0"));
        assert!(!is_newer("", "0.5.0"));
        assert_eq!(parse_version("v1.2.3-beta"), (1, 2, 3));
    }
}
