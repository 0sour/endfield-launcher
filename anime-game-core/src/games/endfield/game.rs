use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::version::Version;
use crate::traits::prelude::*;
use super::api;
use super::consts::*;
use super::crypto;
use super::version_diff::*;

/// Read the local game version from the encrypted `config.ini` file
fn get_version_from_config(path: &Path) -> anyhow::Result<Option<Version>> {
    let config_path = path.join("config.ini");

    if !config_path.exists() {
        return Ok(None);
    }

    let content = crypto::decrypt_file_to_string(&config_path)?;

    for line in content.lines() {
        let line = line.trim();

        if let Some(version) = line.strip_prefix("version=") {
            return Ok(Version::from_str(version.trim()));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    path: PathBuf,
    edition: GameEdition
}

impl GameExt for Game {
    type Edition = GameEdition;

    #[inline]
    fn new(path: impl Into<PathBuf>, edition: GameEdition) -> Self {
        Self {
            path: path.into(),
            edition
        }
    }

    #[inline]
    fn path(&self) -> &Path {
        self.path.as_path()
    }

    #[inline]
    fn edition(&self) -> GameEdition {
        self.edition
    }

    /// Check if the game is installed
    ///
    /// The game is considered installed when both the executable and the
    /// encrypted `config.ini` exist in the game directory.
    fn is_installed(&self) -> bool {
        self.path.join(self.edition.exe_name()).exists()
            && self.path.join("config.ini").exists()
    }

    #[tracing::instrument(level = "trace", ret)]
    /// Try to get latest game version
    fn get_latest_version(edition: GameEdition) -> anyhow::Result<Version> {
        tracing::trace!("Trying to get latest game version");

        // Empty version is fine here: the latest release version is
        // returned regardless of the requested version
        let response = api::request(edition, String::new())?;

        let version = response
            .version
            .ok_or_else(|| anyhow::anyhow!("API response doesn't contain a version"))?;

        Ok(Version::from_str(&version).unwrap())
    }

    #[tracing::instrument(level = "debug", ret)]
    fn get_version(&self) -> anyhow::Result<Version> {
        tracing::debug!("Trying to get installed game version");

        let stored_version_path = self.path.join(".version");
        let stored_version = crate::version_detect::parse_dotversion(&stored_version_path);

        if let Some(version_from_config) = get_version_from_config(&self.path)? {
            tracing::info!(
                version = version_from_config.to_string(),
                "Found game version from config.ini"
            );
            return Ok(version_from_config);
        }

        if let Some(stored_version) = stored_version {
            tracing::info!(version = stored_version.to_string(), "Found stored version");
            return Ok(stored_version);
        }

        tracing::error!("Version wasn't found in config.ini or .version file");

        anyhow::bail!("Version wasn't found in config.ini or .version file");
    }
}

impl Game {
    #[tracing::instrument(level = "debug", ret)]
    pub fn try_get_diff(&self) -> anyhow::Result<VersionDiff> {
        tracing::debug!("Trying to find version diff for the game");

        // Report the currently installed version so that the API can
        // return the predownload (`pre_patch`), which it only does when
        // the client's version matches the latest live release. Uninstalled
        // games don't need it, and a broken installation reports an error
        // below as before
        let requested_version = if self.is_installed() {
            self.get_version()
                .map(|version| version.to_string())
                .unwrap_or_default()
        }
        else {
            String::new()
        };

        let response = api::request(self.edition, requested_version)?;

        let latest_version = response
            .version
            .as_deref()
            .map(Version::from_str)
            .flatten()
            .ok_or_else(|| anyhow::anyhow!("API response doesn't contain a version"))?;

        if self.is_installed() {
            let current = match self.get_version() {
                Ok(version) => version,
                Err(err) => {
                    if self.path.exists() {
                        if !self.path.metadata()?.is_dir() {
                            anyhow::bail!("Path is not a directory: {}", self.path.display());
                        }
                        if self
                            .path
                            .read_dir()
                            .context(format!("Checking game dir: {}", self.path.display()))?
                            .count()
                            > 0
                        {
                            anyhow::bail!("Game directory is not empty")
                        }

                        return Ok(VersionDiff::NotInstalled {
                            latest: latest_version,
                            edition: self.edition,
                            downloaded_size: 0,
                            unpacked_size: 0,
                            segments_uris: Vec::new(),
                            installation_path: Some(self.path.clone()),
                            version_file_path: None,
                            temp_folder: None,
                            password: None
                        });
                    }

                    return Err(err);
                }
            };

            if current >= latest_version {
                tracing::debug!("Game version is latest");

                // Check if there's a predownload available
                if let Some(pre_patch) = response.pre_patch {
                    if let Some(pre_version) = pre_patch.version.as_deref().map(Version::from_str).flatten() {
                        if pre_version > latest_version {
                            let (downloaded_size, unpacked_size, uris) =
                                patch_sizes(&pre_patch);

                            return Ok(VersionDiff::Predownload {
                                current,
                                latest: pre_version,
                                uris,
                                edition: self.edition,
                                downloaded_size,
                                unpacked_size,
                                installation_path: Some(self.path.clone()),
                                version_file_path: None,
                                temp_folder: None,
                                password: pre_patch.cd_key.clone()
                            });
                        }
                    }
                }

                Ok(VersionDiff::Latest {
                    version: current,
                    edition: self.edition
                })
            }
            else {
                tracing::debug!(
                    "Game is outdated: {} -> {}",
                    current,
                    latest_version
                );

                // Check if there's an incremental patch for the current version
                if let Some(patch) = response.patch {
                    // The API doesn't include a version field in the patch
                    // object; a non-empty patches list means the patch
                    // applies to the currently installed version (the API
                    // only returns it for the version we requested)
                    if patch
                        .patches
                        .as_ref()
                        .map(|patches| !patches.is_empty())
                        .unwrap_or(false)
                    {
                        let (downloaded_size, unpacked_size, uris) =
                            patch_sizes(&patch);

                        return Ok(VersionDiff::Diff {
                            current,
                            latest: latest_version,
                            uris,
                            edition: self.edition,
                            downloaded_size,
                            unpacked_size,
                            installation_path: Some(self.path.clone()),
                            version_file_path: None,
                            temp_folder: None,
                            password: patch.cd_key.clone()
                        });
                    }
                }

                Ok(VersionDiff::Outdated {
                    current,
                    latest: latest_version,
                    edition: self.edition
                })
            }
        }
        else {
            tracing::debug!("Game is not installed");

            let (downloaded_size, unpacked_size, uris) = match response.pkg {
                Some(pkg) => {
                    let packs = pkg.packs.unwrap_or_default();

                    let downloaded_size = packs
                        .iter()
                        .flat_map(|pack| pack.package_size.as_deref().and_then(|s| s.parse::<u64>().ok()))
                        .sum();

                    let uris = packs.into_iter().map(|pack| pack.url).collect::<Vec<_>>();

                    (downloaded_size, downloaded_size, uris)
                }
                None => (0, 0, Vec::new())
            };

            Ok(VersionDiff::NotInstalled {
                latest: latest_version,
                edition: self.edition,
                downloaded_size,
                unpacked_size,
                segments_uris: uris,
                installation_path: Some(self.path.clone()),
                version_file_path: None,
                temp_folder: None,
                // Full packages are not encrypted (only incremental patches are)
                password: None
            })
        }
    }
}

/// Calculate download sizes and collect URIs from a patch info
fn patch_sizes(patch: &api::schema::PatchInfo) -> (u64, u64, Vec<String>) {
    let packs = patch.patches.clone().unwrap_or_default();

    let downloaded_size = packs
        .iter()
        .flat_map(|pack| pack.package_size.as_deref().and_then(|s| s.parse::<u64>().ok()))
        .sum();

    let total_size = patch
        .total_size
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(downloaded_size);

    let uris = packs.into_iter().map(|pack| pack.url).collect::<Vec<_>>();

    (downloaded_size, total_size, uris)
}
