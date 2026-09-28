//! Mises à jour : on regarde la dernière release publiée sur GitHub, et si elle est plus récente,
//! on télécharge son installeur puis on le lance. L'installeur remplace l'ancienne version en place.
//!
//! Chaque installeur doit être signé (fichier `.sig` publié à côté, voir `scripts/signature.mjs`) avec
//! une clé qui ne quitte jamais le PC de Kaury Studio. Un installeur sans signature valide n'est jamais
//! lancé : un compte GitHub piraté ou un fichier échangé en route ne suffit pas à installer autre chose.

use crate::langue::tr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Serialize;

const REPO: &str = "theoblondel/kaury-clean";
/// Seuls les installeurs publiés dans les releases de ce repo sont acceptés.
const DOWNLOAD_PREFIX: &str = "https://github.com/theoblondel/kaury-clean/releases/download/";

/// Clé publique de signature des installeurs (la clé privée reste hors de GitHub).
const PUBLIC_KEY: [u8; 32] = [
    0xe1, 0xaf, 0x6b, 0x1a, 0xdb, 0x03, 0x41, 0x5f, 0x0d, 0xce, 0xf9, 0x3f, 0xf5, 0x06, 0x6c, 0x68, 0xba, 0xfd, 0x3c,
    0x00, 0xab, 0xf9, 0xf2, 0xbf, 0x90, 0x5b, 0x31, 0xb2, 0xa4, 0x7d, 0x44, 0x03,
];
/// Doit rester identique à CONTEXT dans scripts/signature.mjs.
const SIGNATURE_CONTEXT: &str = "kaury-clean/installeur/v1\n";
/// Un installeur fait 2 Mo : au-delà de 300 Mo, ce n'est pas le nôtre.
const MAX_INSTALLER: u64 = 300 * 1024 * 1024;

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
    #[serde(skip)]
    pub signature_url: String,
}

/// « v0.10.2 » → (0, 10, 2). Les morceaux illisibles comptent pour 0.
pub fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

/// L'installeur `file_name` est-il bien signé par la clé `key` ? Le nom (qui porte la version) fait
/// partie de ce qui est signé : un vieil installeur signé ne peut pas se faire passer pour un neuf.
pub fn signed_by(key: &[u8; 32], file_name: &str, installer: &[u8], signature: &[u8]) -> bool {
    let (Ok(key), Ok(signature)) = (VerifyingKey::from_bytes(key), Signature::from_slice(signature)) else {
        return false;
    };
    let mut message = Vec::with_capacity(SIGNATURE_CONTEXT.len() + file_name.len() + 1 + installer.len());
    message.extend_from_slice(SIGNATURE_CONTEXT.as_bytes());
    message.extend_from_slice(file_name.as_bytes());
    message.push(b'\n');
    message.extend_from_slice(installer);
    key.verify_strict(&message, &signature).is_ok()
}

/// Connexion chiffrée avec le TLS de Windows : il respecte les certificats et le proxy du PC.
fn agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20));
    if let Ok(tls) = native_tls::TlsConnector::new() {
        builder = builder.tls_connector(std::sync::Arc::new(tls));
    }
    builder.build()
}

fn asset_url(release: &serde_json::Value, name: &str) -> String {
    release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| !name.is_empty() && a["name"].as_str() == Some(name))
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or("")
        .to_string()
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let release: serde_json::Value = agent()
        .get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .set("User-Agent", "KauryClean")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => tr("Aucune version publiée pour l'instant", "No version published yet").to_string(),
            _ => tr("Impossible de joindre GitHub. Vérifie ta connexion.", "Can't reach GitHub. Check your connection.").to_string(),
        })?
        .into_json()
        .map_err(|_| tr("Réponse de GitHub illisible", "Unreadable response from GitHub").to_string())?;

    let latest = release["tag_name"].as_str().unwrap_or("").to_string();
    let asset = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with("-setup.exe")));
    let file_name = asset.and_then(|a| a["name"].as_str()).unwrap_or("").to_string();
    let download_url = asset_url(&release, &file_name);
    // La signature est publiée à côté : « Kaury.Clean_0.8.0_x64-setup.exe.sig ». Tant qu'elle n'est
    // pas là, la mise à jour n'est pas proposée.
    let signature_url = asset_url(&release, &format!("{file_name}.sig"));
    let version = latest.trim_start_matches('v').to_string();

    Ok(UpdateInfo {
        available: is_newer(&latest, current)
            && download_url.starts_with(DOWNLOAD_PREFIX)
            && signature_url.starts_with(DOWNLOAD_PREFIX)
            && !version.is_empty()
            && file_name.contains(&format!("_{version}_")),
        current: current.to_string(),
        latest: version,
        notes: release["body"].as_str().unwrap_or("").to_string(),
        size: asset.and_then(|a| a["size"].as_u64()).unwrap_or(0),
        page: release["html_url"].as_str().unwrap_or("").to_string(),
        file_name,
        download_url,
        signature_url,
    })
}

fn get(url: &str) -> Result<ureq::Response, String> {
    agent().get(url).set("User-Agent", "KauryClean").call().map_err(|_| tr("Le téléchargement a échoué", "The download failed").to_string())
}

/// Ouvre l'installeur en interdisant à tout autre programme de le modifier, de le renommer ou de le
/// supprimer tant que le fichier renvoyé reste ouvert.
#[cfg(windows)]
fn open_locked(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(path)
}

#[cfg(not(windows))]
fn open_locked(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Télécharge l'installeur de la dernière version dans le dossier temporaire et vérifie sa signature.
/// Renvoie le fichier verrouillé : il doit rester ouvert jusqu'au lancement de l'installeur, pour que
/// personne ne l'échange entre la vérification et le lancement.
/// `progress` reçoit le pourcentage téléchargé.
pub fn download(current: &str, progress: &dyn Fn(u32)) -> Result<(File, PathBuf), String> {
    // On redemande la release ici : l'adresse ne vient jamais de l'interface.
    let info = check(current)?;
    if !info.available {
        return Err(tr("Tu as déjà la dernière version", "You already have the latest version").into());
    }

    let mut signature = vec![];
    get(&info.signature_url)?
        .into_reader()
        .take(1024)
        .read_to_end(&mut signature)
        .map_err(|_| tr("Le téléchargement a été coupé", "The download was interrupted").to_string())?;

    let response = get(&info.download_url)?;
    let total: u64 = response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(info.size);
    if total > MAX_INSTALLER {
        return Err(tr("Cette mise à jour est anormalement lourde : elle n'a pas été téléchargée", "This update is abnormally large: it was not downloaded").into());
    }

    let name: String = info.file_name.chars().filter(|c| c.is_ascii_alphanumeric() || "._- ".contains(*c)).collect();
    // Dossier neuf et verrouillé, fichier neuf : rien de préparé à l'avance par un autre programme
    // (un lien vers un fichier de Windows, par exemple) ne peut être écrasé à sa place.
    let (_dir_lock, dir) = crate::garde::private_dir(&std::env::temp_dir()).map_err(|e| e.to_string())?;
    let path = dir.join(name);
    {
        let mut file = File::options().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
        let mut reader = response.into_reader().take(MAX_INSTALLER + 1);
        let mut buffer = [0u8; 64 * 1024];
        let mut done = 0u64;
        let mut shown = u32::MAX;
        loop {
            let n = reader.read(&mut buffer).map_err(|_| tr("Le téléchargement a été coupé", "The download was interrupted").to_string())?;
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
            return Err(tr("Le téléchargement est incomplet", "The download is incomplete").into());
        }
    }

    let refuse = |path: &Path| {
        let _ = std::fs::remove_file(path);
        tr("Cette mise à jour n'a pas pu être vérifiée : elle n'a pas été installée. Réessaie plus tard.", "This update couldn't be verified: it was not installed. Try again later.").to_string()
    };
    // On relit le fichier par le handle verrouillé : c'est exactement ce contenu-là qui sera lancé.
    let mut locked = open_locked(&path).map_err(|_| refuse(&path))?;
    let mut installer = vec![];
    locked.read_to_end(&mut installer).map_err(|_| tr("Lecture de l'installeur impossible", "Can't read the installer").to_string())?;
    if !signed_by(&PUBLIC_KEY, &info.file_name, &installer, &signature) {
        drop(locked);
        return Err(refuse(&path));
    }
    Ok((locked, path))
}

/// Lance l'installeur téléchargé. Windows demande l'autorisation administrateur, puis
/// l'installeur remplace l'ancienne version. À appeler pendant que le fichier est encore verrouillé.
#[cfg(windows)]
pub fn launch_installer(path: &Path) -> Result<(), String> {
    crate::elevation::shell_open(&path.to_string_lossy(), "")
}

#[cfg(not(windows))]
pub fn launch_installer(_path: &Path) -> Result<(), String> {
    Err(tr("Disponible uniquement sur Windows", "Only available on Windows").into())
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

    /// Clé de test (graine [7; 32]) et signature produites avec Node, comme scripts/signature.mjs :
    /// l'appli et le script de signature parlent bien la même langue.
    const TEST_KEY: [u8; 32] = [
        0xea, 0x4a, 0x6c, 0x63, 0xe2, 0x9c, 0x52, 0x0a, 0xbe, 0xf5, 0x50, 0x7b, 0x13, 0x2e, 0xc5, 0xf9, 0x95, 0x47,
        0x76, 0xae, 0xbe, 0xbe, 0x7b, 0x92, 0x42, 0x1e, 0xea, 0x69, 0x14, 0x46, 0xd2, 0x2c,
    ];
    const TEST_SIGNATURE: [u8; 64] = [
        0x9f, 0x94, 0x19, 0x6c, 0x5a, 0xd6, 0x56, 0x3f, 0xae, 0xa4, 0x9d, 0x0d, 0x40, 0xd4, 0xf0, 0xb8, 0xd9, 0x99,
        0x41, 0x7c, 0x28, 0xbe, 0x21, 0xa5, 0xbe, 0x41, 0x1b, 0x2c, 0x6b, 0xff, 0x00, 0xc8, 0x5a, 0xc0, 0xe9, 0x48,
        0x89, 0xcb, 0xa2, 0x2f, 0x1d, 0x6d, 0x9d, 0x7d, 0xaf, 0x40, 0x1f, 0xc4, 0x8c, 0xb0, 0x19, 0xf0, 0x98, 0x44,
        0xad, 0xdb, 0x07, 0xe6, 0x67, 0x08, 0x38, 0xf0, 0x99, 0x01,
    ];
    const NAME: &str = "Kaury.Clean_0.8.0_x64-setup.exe";

    #[test]
    fn accepts_an_installer_signed_by_the_script() {
        assert!(signed_by(&TEST_KEY, NAME, b"installeur de test", &TEST_SIGNATURE));
    }

    #[test]
    fn refuses_tampered_renamed_or_unsigned_installers() {
        assert!(!signed_by(&TEST_KEY, NAME, b"installeur de test + virus", &TEST_SIGNATURE));
        assert!(!signed_by(&TEST_KEY, "Kaury.Clean_0.9.0_x64-setup.exe", b"installeur de test", &TEST_SIGNATURE));
        assert!(!signed_by(&PUBLIC_KEY, NAME, b"installeur de test", &TEST_SIGNATURE));
        assert!(!signed_by(&TEST_KEY, NAME, b"installeur de test", &[]));
    }
}
