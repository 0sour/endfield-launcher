//! Server payload (channel switching) mechanism
//!
//! The CN client has two channels (official and Bilibili) that share the
//! same game files but use different SDK DLLs and platform directories.
//! Switching channels means deploying the channel-specific files from the
//! CDN to the game root directory.
//!
//! The file list comes from the encrypted `game_files` manifest, filtered
//! by the whitelist rules below (reverse engineered from the official
//! launcher by the Xel-Launcher project).

use std::path::{Path, PathBuf};

use super::consts::GameEdition;
use super::crypto;
use super::repairer;

/// Files that are shared between all channels
const COMMON_ROOT_FILES: &[&str] = &["U8CoreUI.dll", "U8SDK.dll", "u8_channel.dll"];

/// Whitelist rules for the payload files of each channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadRules {
    /// Root files to deploy
    pub root_files: Vec<&'static str>,

    /// Directory prefixes to deploy
    pub directory_prefixes: Vec<&'static str>
}

impl GameEdition {
    /// Get the payload whitelist rules for this edition
    pub fn payload_rules(&self) -> PayloadRules {
        match self {
            Self::Official => PayloadRules {
                root_files: vec![
                    "hgsdk.dll",
                    "PlatformProcess.dll",
                    "PlatformProcess.exe",
                    "webviewsdk.dll"
                ],
                directory_prefixes: vec!["sdkdata", "U8Data/config"]
            },

            Self::Bilibili => PayloadRules {
                root_files: vec![
                    "PCGameSDK.dll",
                    "PlatformProcess.dll",
                    "PlatformProcess.exe",
                    "webviewsdk.dll"
                ],
                directory_prefixes: vec!["BLPlatform64", "U8Data/config"]
            }
        }
    }

    /// Check if the given relative path is excluded from deployment
    pub fn is_payload_excluded(path: &str) -> bool {
        let path = path.replace('\\', "/");

        path == "config.ini"
            || path.starts_with("Arknights_Data/")
            || path.starts_with("Endfield_Data/")
            || path == "Arknights.exe"
            || path == "Endfield.exe"
            || path == "GameAssembly.dll"
            || path == "baselib.dll"
            || path.starts_with("UnityPlayer")
            || path.starts_with("game_files")
            || path.starts_with("payload-state.json")
    }

    /// Check if the given relative path matches the payload whitelist
    pub fn is_payload_file(&self, path: &str) -> bool {
        if Self::is_payload_excluded(path) {
            return false;
        }

        let rules = self.payload_rules();
        let path = path.replace('\\', "/");

        // Check root files (common + channel-specific)
        if COMMON_ROOT_FILES.iter().any(|file| *file == path) {
            return true;
        }

        if rules.root_files.iter().any(|file| *file == path) {
            return true;
        }

        // Check directory prefixes
        rules
            .directory_prefixes
            .iter()
            .any(|prefix| path.starts_with(&format!("{prefix}/")))
    }
}

/// Download the payload files for the given edition
///
/// Downloads the whitelisted files from the CDN to the destination folder.
/// Returns the list of downloaded file paths (relative to the game root).
#[cfg(feature = "install")]
pub fn download_payload(
    game_edition: GameEdition,
    dest: impl AsRef<Path>,
    progress: impl Fn(u64, u64) + Send + 'static
) -> anyhow::Result<Vec<PathBuf>> {
    use crate::installer::downloader::Downloader;

    let files = repairer::try_get_integrity_files(game_edition, None)?;

    let payload_files = files
        .into_iter()
        .filter(|file| game_edition.is_payload_file(&file.path.to_string_lossy()))
        .collect::<Vec<_>>();

    let total = payload_files.len() as u64;
    let mut downloaded = 0u64;
    let mut result = Vec::new();

    for file in payload_files {
        let relative = file.path.to_string_lossy().to_string();
        let target = dest.as_ref().join(&relative);

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Skip if the file already exists with the correct MD5
        if target.exists() && file.fast_verify(dest.as_ref()) {
            result.push(PathBuf::from(&relative));
            downloaded += 1;
            (progress)(downloaded, total);
            continue;
        }

        let uri = format!("{}/{}", file.base_url, relative);

        let mut downloader = Downloader::new(uri)?
            .with_free_space_check(false);

        downloader.download(&target, |_, _| {})?;

        if !file.verify(dest.as_ref()) {
            anyhow::bail!("Downloaded payload file failed MD5 verification: {relative}");
        }

        result.push(PathBuf::from(&relative));
        downloaded += 1;
        (progress)(downloaded, total);
    }

    Ok(result)
}

/// Deploy the payload files to the game root directory
///
/// Uses hard links when possible (same filesystem) and falls back to
/// copying otherwise.
#[cfg(feature = "install")]
pub fn deploy_payload(
    payload_dir: impl AsRef<Path>,
    game_dir: impl AsRef<Path>,
    progress: impl Fn(u64, u64) + Send + 'static
) -> anyhow::Result<()> {
    let payload_dir = payload_dir.as_ref();
    let game_dir = game_dir.as_ref();

    let files = list_files(payload_dir)?;
    let total = files.len() as u64;
    let mut deployed = 0u64;

    for file in files {
        let relative = file.strip_prefix(payload_dir)?;
        let target = game_dir.join(relative);

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Try hard link first (same filesystem), fall back to copy
        if let Err(_) = std::fs::hard_link(&file, &target) {
            std::fs::copy(&file, &target)?;
        }

        deployed += 1;
        (progress)(deployed, total);
    }

    Ok(())
}

/// Recursively list all files in a directory
#[cfg(feature = "install")]
fn list_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            files.extend(list_files(&path)?);
        }
        else {
            files.push(path);
        }
    }

    Ok(files)
}

/// Get the payload state (which files are deployed) for the given edition
///
/// Returns the list of relative paths that are currently deployed in the
/// game directory and match the whitelist.
pub fn get_deployed_payload(
    game_edition: GameEdition,
    game_dir: impl AsRef<Path>
) -> anyhow::Result<Vec<PathBuf>> {
    let game_dir = game_dir.as_ref();

    let mut deployed = Vec::new();

    for file in list_files(game_dir)? {
        let relative = file.strip_prefix(game_dir)?;
        let relative_str = relative.to_string_lossy().to_string();

        if game_edition.is_payload_file(&relative_str) {
            deployed.push(relative.to_path_buf());
        }
    }

    Ok(deployed)
}

/// Decrypt and parse the `game_files` manifest from the CDN
///
/// Returns the list of integrity files. This is a convenience wrapper
/// around `repairer::try_get_integrity_files` that also decrypts the
/// manifest (the manifest itself is encrypted with AES-256-CBC).
pub fn get_game_files_manifest(
    game_edition: GameEdition
) -> anyhow::Result<Vec<crate::repairer::IntegrityFile>> {
    repairer::try_get_integrity_files(game_edition, None)
}

// Re-export crypto for convenience
pub use crypto::decrypt_bytes_to_string;
