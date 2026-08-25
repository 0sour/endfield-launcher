use std::path::{Path, PathBuf};

use cached::proc_macro::cached;

use super::api;
use super::consts::GameEdition;
use super::crypto;
use crate::repairer::IntegrityFile;

/// Get the resource base URL from the API response
fn get_resource_base_url(game_edition: GameEdition) -> anyhow::Result<String> {
    let response = api::request(game_edition)?;

    response
        .pkg
        .and_then(|pkg| pkg.file_path)
        .ok_or_else(|| anyhow::anyhow!("API response doesn't contain a resource base URL"))
}

/// Download and parse the encrypted `game_files` manifest
///
/// The manifest is a JSONL file where each line is
/// `{"path":"...","md5":"...","size":...}`.
#[cached(result)]
pub fn try_get_integrity_files(
    game_edition: GameEdition,
    timeout: Option<u64>
) -> anyhow::Result<Vec<IntegrityFile>> {
    let base_url = get_resource_base_url(game_edition)?;

    let response = minreq::get(format!("{base_url}/game_files"))
        .with_timeout(timeout.unwrap_or(*crate::REQUESTS_TIMEOUT))
        .send()?;

    let encrypted = response.as_bytes();

    let content = crypto::decrypt_bytes_to_string(encrypted)?;

    let mut files = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let entry: api::schema::GameFileEntry = serde_json::from_str(line)?;

        files.push(IntegrityFile {
            path: PathBuf::from(entry.path),
            md5: entry.md5,
            size: entry.size as u64,
            base_url: base_url.clone()
        });
    }

    Ok(files)
}

/// Find a single integrity file by its relative path
pub fn try_get_integrity_file(
    game_edition: GameEdition,
    relative_path: impl AsRef<str>,
    timeout: Option<u64>
) -> anyhow::Result<Option<IntegrityFile>> {
    let files = try_get_integrity_files(game_edition, timeout)?;

    Ok(files
        .into_iter()
        .find(|file| file.path == Path::new(relative_path.as_ref())))
}

/// Get the list of unused files in the game directory
pub fn try_get_unused_files(
    game_edition: GameEdition,
    game_dir: impl AsRef<Path>,
    timeout: Option<u64>
) -> anyhow::Result<Vec<PathBuf>> {
    let files = try_get_integrity_files(game_edition, timeout)?;

    let used_files = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    let skip_names = [
        "webCaches",
        "SDKCaches",
        "GeneratedSoundBanks",
        "ScreenShot",
        "Diffs",
        "config.ini",
        ".version"
    ]
    .into_iter()
    .map(|name| name.to_string())
    .collect::<Vec<_>>();

    crate::repairer::try_get_unused_files(game_dir.as_ref(), used_files, skip_names)
}
