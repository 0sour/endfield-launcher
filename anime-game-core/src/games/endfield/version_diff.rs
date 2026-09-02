use std::path::{Path, PathBuf};
use std::os::unix::prelude::PermissionsExt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::consts::GameEdition;
use crate::version::Version;
use crate::traits::version_diff::VersionDiffExt;
#[cfg(feature = "install")]
use crate::{
    external::hpatchz,
    installer::{
        archives::Archive,
        downloader::{Downloader, DownloadingError},
        free_space,
        installer::Update as InstallerUpdate
    }
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffUpdate {
    CheckingFreeSpace(PathBuf),

    InstallerUpdate(InstallerUpdate),

    ApplyingHdiffStarted,
    ApplyingHdiffProgress(u64, u64),
    ApplyingHdiffFinished,

    RemovingOutdatedStarted,
    RemovingOutdatedProgress(u64, u64),
    RemovingOutdatedFinished
}

impl From<InstallerUpdate> for DiffUpdate {
    #[inline]
    fn from(update: InstallerUpdate) -> Self {
        Self::InstallerUpdate(update)
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffDownloadingError {
    /// Your installation is already up to date and not needed to be updated
    #[error("Component version is already latest")]
    AlreadyLatest,

    /// Current version is too outdated and can't be updated.
    /// It means that you have to download everything from zero
    #[error("Components version is too outdated and can't be updated")]
    Outdated,

    /// When there's multiple urls and you can't save them as a single file
    #[error("Component has multiple downloading urls and can't be saved as a single file")]
    MultipleSegments,

    /// Failed to fetch remove data. Redirected from `Downloader`
    #[error("{0}")]
    DownloadingError(#[from] DownloadingError),

    /// Failed to apply hdiff patch
    #[error("Failed to apply hdiff patch: {0}")]
    HdiffPatch(String),

    /// Installation path wasn't specified
    #[error("Path to the component's downloading folder is not specified")]
    PathNotSpecified
}

impl From<minreq::Error> for DiffDownloadingError {
    fn from(error: minreq::Error) -> Self {
        DownloadingError::Minreq(error.to_string()).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionDiff {
    /// Latest version
    Latest {
        version: Version,
        edition: GameEdition
    },

    /// Component's update can be predownloaded, but you still can use it
    Predownload {
        current: Version,
        latest: Version,

        uris: Vec<String>,
        edition: GameEdition,

        downloaded_size: u64,
        unpacked_size: u64,

        installation_path: Option<PathBuf>,
        version_file_path: Option<PathBuf>,
        temp_folder: Option<PathBuf>,

        /// Password for the encrypted update archives (from the API's `cd_key`)
        password: Option<String>
    },

    /// Component should be updated before using it
    Diff {
        current: Version,
        latest: Version,

        uris: Vec<String>,
        edition: GameEdition,

        downloaded_size: u64,
        unpacked_size: u64,

        installation_path: Option<PathBuf>,
        version_file_path: Option<PathBuf>,
        temp_folder: Option<PathBuf>,

        /// Password for the encrypted update archives (from the API's `cd_key`)
        password: Option<String>
    },

    /// Difference can't be calculated because installed version is too old
    Outdated {
        current: Version,
        latest: Version,
        edition: GameEdition
    },

    /// Component is not yet installed
    NotInstalled {
        latest: Version,
        segments_uris: Vec<String>,
        edition: GameEdition,

        downloaded_size: u64,
        unpacked_size: u64,

        installation_path: Option<PathBuf>,
        version_file_path: Option<PathBuf>,
        temp_folder: Option<PathBuf>,

        /// Password for the encrypted install archives (from the API's `cd_key`)
        password: Option<String>
    }
}

impl VersionDiff {
    /// Get the password for the encrypted update archives
    ///
    /// Return `None` if the archives are not encrypted
    pub fn password(&self) -> Option<String> {
        match self {
            // Can't be installed
            Self::Latest { .. } | Self::Outdated { .. } => None,

            // Can be installed
            Self::Predownload { password, .. }
            | Self::Diff { password, .. }
            | Self::NotInstalled { password, .. } => password.to_owned()
        }
    }

    /// Get `.version` file path
    pub fn version_file_path(&self) -> Option<PathBuf> {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => None,

            // Can be installed
            Self::Predownload {
                version_file_path, ..
            }
            | Self::Diff {
                version_file_path, ..
            }
            | Self::NotInstalled {
                version_file_path, ..
            } => version_file_path.to_owned()
        }
    }

    /// Return currently selected temp folder path
    ///
    /// Default is `std::env::temp_dir()` value
    pub fn temp_folder(&self) -> PathBuf {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => std::env::temp_dir(),

            // Can be installed
            Self::Predownload {
                temp_folder, ..
            }
            | Self::Diff {
                temp_folder, ..
            }
            | Self::NotInstalled {
                temp_folder, ..
            } => match temp_folder {
                Some(path) => path.to_owned(),
                None => std::env::temp_dir()
            }
        }
    }

    pub fn with_temp_folder(mut self, temp: PathBuf) -> Self {
        match &mut self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => self,

            // Can be installed
            Self::Predownload {
                temp_folder, ..
            }
            | Self::Diff {
                temp_folder, ..
            }
            | Self::NotInstalled {
                temp_folder, ..
            } => {
                *temp_folder = Some(temp);

                self
            }
        }
    }
}

impl VersionDiffExt for VersionDiff {
    type Edition = GameEdition;
    type Error = DiffDownloadingError;
    type Update = DiffUpdate;

    /// Check whether all segments of the diff are already downloaded
    /// into the given folder
    #[cfg(feature = "install")]
    fn is_downloaded(&self, folder: impl AsRef<Path>) -> bool {
        let uris = match self {
            // Can't be downloaded at all
            Self::Latest { .. } | Self::Outdated { .. } => return false,

            // Can be downloaded
            Self::Predownload { uris, .. } | Self::Diff { uris, .. } => uris,

            // Can be installed but requires install_to logic
            Self::NotInstalled {
                ..
            } => return false
        };

        uris.iter().all(|uri| {
            let filename = Downloader::new(uri)
                .map(|downloader| downloader.get_filename().to_string())
                .unwrap_or_default();

            folder.as_ref().join(filename).exists()
        })
    }

    /// Download all segments of the diff into the given folder
    ///
    /// Endfield distributes its updates as multiple ZIP volumes
    /// (e.g. `.zip.001` ... `.zip.NNN`), so `download_as` (which the
    /// default `download_to` delegates to) would only get the first one.
    #[cfg(feature = "install")]
    fn download_to(
        &mut self,
        folder: impl AsRef<Path>,
        progress: impl Fn(u64, u64) + Send + 'static
    ) -> Result<(), Self::Error> {
        tracing::debug!("Downloading version difference segments");

        // Already fully downloaded => nothing to do
        if self.is_downloaded(&folder) {
            if let Some(size) = self.downloaded_size() {
                tracing::info!("All diff segments are already downloaded, reporting full progress");
                (progress)(size, size);
            }
            return Ok(());
        }

        let (uris, downloaded_size) = match self {
            // Can't be downloaded
            Self::Latest { .. } => return Err(Self::Error::AlreadyLatest),
            Self::Outdated { .. } => return Err(Self::Error::Outdated),

            // Can be downloaded
            Self::Predownload { uris, downloaded_size, .. }
            | Self::Diff { uris, downloaded_size, .. } => (uris, *downloaded_size),

            // Can be installed but requires install_to logic
            Self::NotInstalled { .. } => return Err(Self::Error::MultipleSegments)
        };

        let folder = folder.as_ref().to_path_buf();

        // The progress callback is used from multiple segment downloads,
        // so it needs to be shared between them
        let progress = std::sync::Arc::new(std::sync::Mutex::new(progress));

        let mut current_downloaded = 0;

        for uri in uris {
            let mut downloader = Downloader::new(uri)?
                // Continue downloading from where we left off (resume support)
                .with_continue_downloading(true);

            let segment_name = downloader.get_filename().to_string();

            // Skip segments that are already fully downloaded
            let target = folder.join(&segment_name);

            // Migrate files saved with old names (before query parameters
            // were stripped from the filename) so their downloads can resume
            // instead of starting over
            migrate_legacy_segment(&folder, &segment_name);

            if is_segment_downloaded(&downloader, &target) {
                tracing::info!("Segment already downloaded, skipping: {segment_name}");
                current_downloaded += downloader.length().unwrap_or(0);
                (progress.lock().unwrap())(current_downloaded, downloaded_size);
                continue;
            }

            let local_total = downloader.length().unwrap_or(0);

            let progress = std::sync::Arc::clone(&progress);

            downloader.download(&target, move |curr, _| {
                (progress.lock().unwrap())(current_downloaded + curr, downloaded_size);
            })?;

            current_downloaded += local_total;
        }

        // Report 100% download progress (just in case)
        (progress.lock().unwrap())(downloaded_size, downloaded_size);

        Ok(())
    }

    fn edition(&self) -> GameEdition {
        match self {
            Self::Latest {
                edition, ..
            }
            | Self::Predownload {
                edition, ..
            }
            | Self::Diff {
                edition, ..
            }
            | Self::Outdated {
                edition, ..
            }
            | Self::NotInstalled {
                edition, ..
            } => *edition
        }
    }

    fn current(&self) -> Option<Version> {
        match self {
            Self::Latest {
                version: current, ..
            }
            | Self::Predownload {
                current, ..
            }
            | Self::Diff {
                current, ..
            }
            | Self::Outdated {
                current, ..
            } => Some(*current),

            Self::NotInstalled {
                ..
            } => None
        }
    }

    fn latest(&self) -> Version {
        match self {
            Self::Latest {
                version: latest, ..
            }
            | Self::Predownload {
                latest, ..
            }
            | Self::Diff {
                latest, ..
            }
            | Self::Outdated {
                latest, ..
            }
            | Self::NotInstalled {
                latest, ..
            } => *latest
        }
    }

    fn downloaded_size(&self) -> Option<u64> {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => None,

            // Can be installed
            Self::Predownload {
                downloaded_size, ..
            }
            | Self::Diff {
                downloaded_size, ..
            }
            | Self::NotInstalled {
                downloaded_size, ..
            } => Some(*downloaded_size)
        }
    }

    fn unpacked_size(&self) -> Option<u64> {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => None,

            // Can be installed
            Self::Predownload {
                unpacked_size, ..
            }
            | Self::Diff {
                unpacked_size, ..
            }
            | Self::NotInstalled {
                unpacked_size, ..
            } => Some(*unpacked_size)
        }
    }

    fn installation_path(&self) -> Option<&Path> {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => None,

            // Can be installed
            Self::Predownload {
                installation_path, ..
            }
            | Self::Diff {
                installation_path, ..
            }
            | Self::NotInstalled {
                installation_path, ..
            } => match installation_path {
                Some(path) => Some(path.as_path()),
                None => None
            }
        }
    }

    fn downloading_uri(&self) -> Option<String> {
        match self {
            // Can't be installed
            Self::Latest {
                ..
            }
            | Self::Outdated {
                ..
            } => None,

            // Can be installed
            Self::Predownload {
                uris, ..
            }
            | Self::Diff {
                uris, ..
            } => uris.first().cloned(),

            // Can be installed but amogus
            Self::NotInstalled {
                ..
            } => None
        }
    }

    fn download_as(
        &mut self,
        path: impl AsRef<Path>,
        progress: impl Fn(u64, u64) + Send + 'static
    ) -> Result<(), Self::Error> {
        tracing::debug!("Downloading version difference");

        let mut downloader = Downloader::new(match self {
            // Can't be downloaded
            Self::Latest {
                ..
            } => return Err(Self::Error::AlreadyLatest),
            Self::Outdated {
                ..
            } => return Err(Self::Error::Outdated),

            // Can be downloaded
            Self::Predownload {
                uris, ..
            }
            | Self::Diff {
                uris, ..
            } => match uris.first() {
                Some(uri) => uri,
                None => return Err(Self::Error::MultipleSegments)
            },

            // Can be installed but amogus
            Self::NotInstalled {
                ..
            } => return Err(Self::Error::MultipleSegments)
        })?;

        if let Err(err) = downloader.download(path.as_ref(), progress) {
            tracing::error!("Failed to download version difference: {err}");

            return Err(err.into());
        }

        Ok(())
    }

    fn install_to(
        &self,
        path: impl AsRef<Path>,
        _thread_count: usize,
        updater: impl Fn(Self::Update) + Clone + Send + 'static
    ) -> Result<(), Self::Error> {
        tracing::debug!("Installing version difference");

        let password = self.password();

        let uris = match self {
            // Can't be installed
            Self::Latest {
                ..
            } => return Err(Self::Error::AlreadyLatest),
            Self::Outdated {
                ..
            } => return Err(Self::Error::Outdated),

            // Can be installed
            Self::Predownload {
                uris, ..
            }
            | Self::Diff {
                uris, ..
            } => uris.to_owned(),

            Self::NotInstalled {
                segments_uris, ..
            } => segments_uris.to_owned()
        };

        let path = path.as_ref().to_path_buf();
        let temp_folder = self.temp_folder();

        let downloaded_size = self
            .downloaded_size()
            .expect("Failed to retrieve downloaded size");
        let unpacked_size = self
            .unpacked_size()
            .expect("Failed to retrieve unpacked size");

        (updater)(DiffUpdate::CheckingFreeSpace(temp_folder.clone()));

        // Check available free space for archive itself
        let Some(space) = free_space::available(&temp_folder)
        else {
            tracing::error!("Path is not mounted: {:?}", temp_folder);

            return Err(DownloadingError::PathNotMounted(temp_folder).into());
        };

        // We can possibly store downloaded archive + unpacked data on the same disk
        let required = if free_space::is_same_disk(&temp_folder, &path) {
            downloaded_size + unpacked_size
        }
        else {
            downloaded_size
        };

        if space < required {
            tracing::error!(
                "No free space available in the temp folder. Required: {required}. Available: {space}"
            );

            return Err(DownloadingError::NoSpaceAvailable(temp_folder, required, space).into());
        }

        (updater)(DiffUpdate::CheckingFreeSpace(path.clone()));

        // Check available free space for unpacked archive data
        let Some(space) = free_space::available(&path)
        else {
            tracing::error!("Path is not mounted: {:?}", &path);

            return Err(DownloadingError::PathNotMounted(path.to_path_buf()).into());
        };

        // We can possibly store downloaded archive + unpacked data on the same disk
        let required = if free_space::is_same_disk(&path, &temp_folder) {
            unpacked_size + downloaded_size
        }
        else {
            unpacked_size
        };

        if space < required {
            tracing::error!(
                "No free space available in the installation folder. Required: {required}. Available: {space}"
            );

            return Err(
                DownloadingError::NoSpaceAvailable(path.to_path_buf(), required, space).into()
            );
        }

        let mut current_downloaded = 0;
        let mut segments_names = Vec::new();

        // Imitate Installer update message
        (updater)(DiffUpdate::InstallerUpdate(
            InstallerUpdate::DownloadingStarted(temp_folder.to_path_buf())
        ));

        // Download segments
        for uri in uris {
            let installer_updater = updater.clone();

            let mut downloader = Downloader::new(uri)?
                // Don't perform space checks because we've already done it
                .with_free_space_check(false);

            let local_total = downloader.length().unwrap();
            let segment_name = downloader.get_filename().to_string();

            // Download segment
            downloader.download(temp_folder.join(&segment_name), move |current, _| {
                (installer_updater)(DiffUpdate::InstallerUpdate(
                    InstallerUpdate::DownloadingProgress(
                        current_downloaded + current,
                        downloaded_size
                    )
                ));
            })?;

            segments_names.push(segment_name);

            current_downloaded += local_total;
        }

        // Report 100% download progress (just in case)
        (updater)(DiffUpdate::InstallerUpdate(
            InstallerUpdate::DownloadingProgress(downloaded_size, downloaded_size)
        ));

        let first_segment_name = segments_names[0].clone();

        // Imitate Installer update message
        (updater)(DiffUpdate::InstallerUpdate(
            InstallerUpdate::DownloadingFinished
        ));

        // Backup the existing config.ini before extraction, so we can
        // restore it if the new one turns out to be invalid
        let config_backup = path.join("config.ini.bak");

        if path.join("config.ini").exists() {
            #[allow(unused_must_use)]
            {
                std::fs::copy(path.join("config.ini"), &config_backup);
            }
        }

        // Extract downloaded segments
        match Archive::open_with_password(
            temp_folder.join(&first_segment_name),
            self.password().as_deref()
        ) {
            Ok(mut archive) => {
                // Temporary workaround as we can't get archive extraction process
                // directly - we'll spawn it in another thread and check this archive entries
                // appearance in the filesystem
                let mut total = 0;

                let entries = archive
                    .get_entries()
                    .expect("Failed to get archive entries");

                // Snapshot the entry names before `entries` is moved into
                // the progress-polling thread; used to verify that the
                // extraction actually produced files
                let entry_names = entries
                    .iter()
                    .map(|entry| entry.name.clone())
                    .collect::<Vec<_>>();

                for entry in &entries {
                    total += entry.size.get_size();

                    let path = path.join(&entry.name);

                    // Failed to change permissions => likely patch-related file and was made by the
                    // sudo, so root
                    #[allow(unused_must_use)]
                    if std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666))
                        .is_err()
                    {
                        // For weird reason we can delete files made by root, but can't modify their
                        // permissions We're not checking its result because
                        // if it's error - then it's either couldn't be removed (which is not the
                        // case) or the file doesn't exist, which we
                        // obviously can just ignore
                        std::fs::remove_file(&path);
                    }
                }

                tracing::trace!("Extracting archive");

                let unpacking_path = path.clone();
                let unpacking_updater = updater.clone();

                let handle_2 = std::thread::spawn(move || {
                    let mut entries = entries
                        .into_iter()
                        .map(|entry| {
                            (
                                unpacking_path.join(&entry.name),
                                entry.size.get_size(),
                                true
                            )
                        })
                        .collect::<Vec<_>>();

                    let mut unpacked = 0;

                    // Give up after 30 minutes: a failed extraction would
                    // otherwise make this thread (and the whole update)
                    // hang forever waiting for files that never appear
                    let deadline = std::time::Instant::now()
                        + std::time::Duration::from_secs(30 * 60);

                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(250));

                        let mut empty = true;

                        for (path, size, remained) in &mut entries {
                            if *remained {
                                empty = false;

                                if std::path::Path::new(path).exists() {
                                    *remained = false;

                                    unpacked += *size;
                                }
                            }
                        }

                        (unpacking_updater)(DiffUpdate::InstallerUpdate(
                            InstallerUpdate::UnpackingProgress(unpacked, total)
                        ));

                        if empty {
                            break;
                        }

                        if std::time::Instant::now() > deadline {
                            tracing::error!(
                                "Extraction progress polling timed out after 30 minutes"
                            );

                            break;
                        }
                    }
                });

                let unpacking_updater = updater.clone();
                let extract_to = path.clone();
                let extract_from = temp_folder.clone();
                let entry_names = entry_names.clone();

                // Run archive extraction in another thread to not to freeze the current one
                let handle_1 = std::thread::spawn(move || {
                    (unpacking_updater)(DiffUpdate::InstallerUpdate(
                        InstallerUpdate::UnpackingStarted(extract_to.clone())
                    ));

                    // We have to create new instance of Archive here
                    // because otherwise it may not work after get_entries method call
                    match Archive::open_with_password(
                        extract_from.join(first_segment_name),
                        password.as_deref()
                    ) {
                        Ok(mut archive) => match archive.extract(&extract_to) {
                            Ok(_) => {
                                // Verify that the extraction actually produced files:
                                // a failed 7z run used to be reported as success,
                                // which then deleted the segments for nothing
                                let extracted = entry_names
                                    .iter()
                                    .filter(|name| extract_to.join(name).exists())
                                    .count();

                                if extracted == 0 {
                                    (unpacking_updater)(DiffUpdate::InstallerUpdate(
                                        InstallerUpdate::UnpackingError(
                                            "Extraction finished but no files were produced".to_string()
                                        )
                                    ));

                                    return;
                                }

                                // Only delete the downloaded segments after a
                                // successful extraction
                                #[allow(unused_must_use)]
                                {
                                    for name in segments_names {
                                        std::fs::remove_file(extract_from.join(name));
                                    }
                                }

                                (unpacking_updater)(DiffUpdate::InstallerUpdate(
                                    InstallerUpdate::UnpackingFinished
                                ));
                            }

                            Err(err) => (unpacking_updater)(DiffUpdate::InstallerUpdate(
                                InstallerUpdate::UnpackingError(err.to_string())
                            ))
                        },

                        Err(err) => (unpacking_updater)(DiffUpdate::InstallerUpdate(
                            InstallerUpdate::UnpackingError(err.to_string())
                        ))
                    }
                });

                handle_1.join().unwrap();
                handle_2.join().unwrap();
            }

            Err(err) => (updater)(DiffUpdate::InstallerUpdate(
                InstallerUpdate::UnpackingError(err.to_string())
            ))
        }

        // Imitate Installer update message
        (updater)(DiffUpdate::InstallerUpdate(
            InstallerUpdate::UnpackingFinished
        ));

        // Validate the new config.ini: it must be decryptable and contain
        // a version line. If it's invalid, restore the backup.
        let config_path = path.join("config.ini");

        if config_path.exists() && !super::crypto::is_valid_config(&config_path) {
            tracing::warn!("New config.ini is invalid, restoring the backup");

            if config_backup.exists() {
                std::fs::copy(&config_backup, &config_path)
                    .map_err(|err| DiffDownloadingError::HdiffPatch(err.to_string()))?;
            }
        }

        // Create `.version` file here even if hdiff patching is failed because
        // it's easier to explain user why he should run files repairer than
        // why he should re-download entire game update because something is failed
        #[allow(unused_must_use)]
        {
            let version_path = self.version_file_path().unwrap_or(path.join(".version"));

            std::fs::write(version_path, self.latest().to_string());
        }

        // Apply delta patches from the patch.json manifest
        apply_delta_patches(&path, &temp_folder, &updater)?;

        tracing::debug!("Deleting outdated files");

        // Remove outdated files
        // We're ignoring Err because in practice it means that deletefiles.txt is
        // missing
        if let Ok(files) = std::fs::read_to_string(path.join("delete_files.txt")) {
            let files = files.lines().collect::<Vec<&str>>();
            let files_len = files.len() as u64;

            (updater)(Self::Update::RemovingOutdatedStarted);

            for (i, file) in files.into_iter().enumerate() {
                let file = path.join(file);

                std::fs::remove_file(&file)
                    .expect(&format!("Failed to remove outdated file: {:?}", file));

                (updater)(Self::Update::RemovingOutdatedProgress(
                    i as u64 + 1,
                    files_len
                ));
            }

            std::fs::remove_file(path.join("delete_files.txt"))
                .expect("Failed to remove delete_files.txt");

            (updater)(Self::Update::RemovingOutdatedFinished);
        }

        // Remove the config.ini backup after a successful update
        #[allow(unused_must_use)]
        {
            std::fs::remove_file(&config_backup);
        }

        Ok(())
    }
}

/// Apply the VFS delta patch pipeline described by `patch.json`
///
/// The manifest is either extracted from the downloaded package or fetched
/// from the API's `v2_patch_info_url`. Each file entry is handled by its
/// `diffType`:
/// - with `local_path`: static copy from the extracted package
/// - with `patch` nodes: hdiff patch applied to the base file
/// - otherwise: verify only
#[cfg(feature = "install")]
fn apply_delta_patches(
    path: &Path,
    _temp_folder: &Path,
    updater: &(impl Fn(DiffUpdate) + Clone + Send + 'static)
) -> Result<(), DiffDownloadingError> {
    use super::api::schema::PatchManifest;

    // Try to read the manifest from the extracted package first
    let local_manifest_path = path.join("patch.json");
    let manifest: Option<PatchManifest> = if local_manifest_path.exists() {
        match std::fs::read_to_string(&local_manifest_path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => None
        }
    }
    else {
        None
    };

    // The update package extracts into the game root with this layout:
    //
    // ```
    // vfs_files/
    // ├── files/Endfield_Data/...          <- static files (local_path)
    // └── vfs_patch/diff_<ver>/...         <- hdiff patches (patch)
    // ```
    //
    // The manifest paths are relative to the package root: `local_path`
    // entries already include the `vfs_files/` prefix, while `patch` paths
    // are bare (`diff_<ver>/...`) and live under `vfs_files/vfs_patch/`.
    let package_root = path.join("vfs_files");
    let patch_root = package_root.join("vfs_patch");

    let resolve_package_path = |relative: &str| -> PathBuf {
        let candidate = path.join(relative);

        if candidate.exists() {
            candidate
        }
        else if relative.starts_with("diff_") {
            patch_root.join(relative)
        }
        else {
            package_root.join(relative)
        }
    };

    let Some(manifest) = manifest else {
        tracing::debug!("No patch.json found, assuming static-only delta update");

        return Ok(());
    };

    let Some(files) = manifest.files else {
        tracing::debug!("Patch manifest has no files, skipping");

        return Ok(());
    };

    let vfs_base_path = manifest
        .vfs_base_path
        .unwrap_or_else(|| concat!("Ark", "nights", "_Data/StreamingAssets/AB/Windows").to_string());

    let source_vfs_base = path.join(&vfs_base_path);
    let target_vfs_base = path.join(&vfs_base_path);

    let total_patch_size = files
        .iter()
        .filter(|f| f.local_path.is_some() || f.patch.is_some())
        .flat_map(|f| f.size)
        .sum::<i64>() as u64;

    let mut current_patched_size = 0u64;

    (updater)(DiffUpdate::ApplyingHdiffStarted);

    for file_node in &files {
        let Some(name) = &file_node.name else {
            continue;
        };

        let target_file = target_vfs_base.join(name);
        let target_dir = target_file.parent().unwrap_or(&target_vfs_base);

        std::fs::create_dir_all(target_dir)
            .map_err(|err| DiffDownloadingError::HdiffPatch(err.to_string()))?;

        // Case 1: static copy from the extracted package
        if let Some(local_path) = &file_node.local_path {
            let source_file = resolve_package_path(local_path);

            if !source_file.exists() {
                tracing::warn!("local_path missing: {local_path}");

                continue;
            }

            #[allow(unused_must_use)]
            {
                std::fs::remove_file(&target_file);
            }

            std::fs::copy(&source_file, &target_file)
                .map_err(|err| DiffDownloadingError::HdiffPatch(err.to_string()))?;

            if let Some(md5) = &file_node.md5 {
                if !check_md5(&target_file, md5) {
                    tracing::warn!("copied file md5 mismatch: {name}");

                    continue;
                }
            }

            current_patched_size += file_node.size.unwrap_or(0) as u64;
            (updater)(DiffUpdate::ApplyingHdiffProgress(
                current_patched_size,
                total_patch_size
            ));

            continue;
        }

        // Case 2: hdiff patch
        if let Some(patch_nodes) = &file_node.patch {
            if let Some(patch_node) = patch_nodes.first() {
                let Some(base_file) = &patch_node.base_file else {
                    continue;
                };
                let Some(patch_path) = &patch_node.patch else {
                    continue;
                };

                let base_file_path = source_vfs_base.join(base_file);
                let diff_file_path = resolve_package_path(patch_path);

                if !base_file_path.exists() {
                    tracing::warn!("base file missing: {base_file}");

                    continue;
                }

                if !diff_file_path.exists() {
                    tracing::warn!("diff file missing: {patch_path}");

                    continue;
                }

                if let Some(base_md5) = &patch_node.base_md5 {
                    if !check_md5(&base_file_path, base_md5) {
                        return Err(DiffDownloadingError::HdiffPatch(format!(
                            "Base file MD5 mismatch: {base_file}"
                        )));
                    }
                }

                // Empty patch file means the base file is the target
                if diff_file_path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
                    if base_file_path != target_file {
                        #[allow(unused_must_use)]
                        {
                            std::fs::remove_file(&target_file);
                        }

                        std::fs::copy(&base_file_path, &target_file)
                            .map_err(|err| DiffDownloadingError::HdiffPatch(err.to_string()))?;
                    }
                }
                else {
                    let temp_out = target_file.with_extension("tmp");

                    #[allow(unused_must_use)]
                    {
                        std::fs::remove_file(&temp_out);
                    }

                    if let Err(err) = hpatchz::patch(&base_file_path, &diff_file_path, &temp_out) {
                        tracing::error!("Failed to apply hdiff patch for {name}: {err}");

                        return Err(DiffDownloadingError::HdiffPatch(err.to_string()));
                    }

                    if let Some(md5) = &file_node.md5 {
                        if !check_md5(&temp_out, md5) {
                            #[allow(unused_must_use)]
                            {
                                std::fs::remove_file(&temp_out);
                            }

                            return Err(DiffDownloadingError::HdiffPatch(format!(
                                "Patched file MD5 mismatch: {name}"
                            )));
                        }
                    }

                    #[allow(unused_must_use)]
                    {
                        std::fs::remove_file(&target_file);
                    }

                    std::fs::rename(&temp_out, &target_file)
                        .map_err(|err| DiffDownloadingError::HdiffPatch(err.to_string()))?;
                }

                current_patched_size += file_node.size.unwrap_or(0) as u64;
                (updater)(DiffUpdate::ApplyingHdiffProgress(
                    current_patched_size,
                    total_patch_size
                ));

                continue;
            }
        }

        // Case 3: verify only
        tracing::debug!("[VerifyOnly] {name}");
    }

    // Remove the manifest file after successful patching
    #[allow(unused_must_use)]
    {
        std::fs::remove_file(&local_manifest_path);
    }

    (updater)(DiffUpdate::ApplyingHdiffFinished);

    Ok(())
}

/// Check if the file's MD5 matches the expected value
#[cfg(feature = "install")]
fn check_md5(path: &Path, expected: &str) -> bool {
    use md5::{Digest, Md5};

    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };

    let mut hasher = Md5::new();
    hasher.update(&bytes);

    let digest = hasher.finalize();

    format!("{digest:x}") == expected.to_ascii_lowercase()
}

/// Check whether the segment file is already fully downloaded
///
/// Compares the local file size against the remote content length
/// (used for resume support: partially downloaded segments are re-downloaded
/// from where they left off by `Downloader`).
#[cfg(feature = "install")]
fn is_segment_downloaded(downloader: &Downloader, target: &Path) -> bool {
    let Some(length) = downloader.length() else {
        return target.exists();
    };

    target.metadata().map(|meta| meta.len() >= length).unwrap_or(false)
}

/// Rename a segment file that was previously saved with URL query
/// parameters in its name (e.g. `file.zip.001?auth_key=...`) to the clean
/// filename, so resume logic can find it
#[cfg(feature = "install")]
fn migrate_legacy_segment(folder: &Path, clean_name: &str) {
    let _ = std::fs::read_dir(folder).map(|entries| {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Match the clean name followed by `?` (query parameters)
            if name.starts_with(clean_name) && name.len() > clean_name.len() {
                let legacy = entry.path();
                let target = folder.join(clean_name);

                tracing::info!("Migrating legacy segment filename: {name}");

                #[allow(unused_must_use)]
                {
                    std::fs::rename(&legacy, &target);
                }
            }
        }
    });
}
