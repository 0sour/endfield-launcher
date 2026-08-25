//! Account management (sdk_data backup/restore)
//!
//! The game stores its login session data in the `sdk_data_*` directory
//! under `AppData/LocalLow/Hypergryph/Arknights` (inside the Wine prefix).
//! Switching accounts means backing up the current `sdk_data_*` directory
//! and restoring another one.

use std::path::{Path, PathBuf};

/// Get the game's app data directory inside the Wine prefix
///
/// Returns the first `sdk_data_*` directory found, or `None` if the game
/// hasn't been run yet.
pub fn get_sdk_data_dir(prefix: impl AsRef<Path>) -> anyhow::Result<Option<PathBuf>> {
    let users_dir = prefix.as_ref().join("drive_c/users");

    if !users_dir.exists() {
        return Ok(None);
    }

    for entry in std::fs::read_dir(&users_dir)? {
        let entry = entry?;

        if !entry.file_type()?.is_dir() {
            continue;
        }

        let app_data = entry
            .path()
            .join("AppData/LocalLow/Hypergryph/Arknights");

        if !app_data.exists() {
            continue;
        }

        // Find the first sdk_data_* directory
        for sub in std::fs::read_dir(&app_data)? {
            let sub = sub?;
            let name = sub.file_name().to_string_lossy().to_string();

            if sub.file_type()?.is_dir() && name.starts_with("sdk_data_") {
                return Ok(Some(sub.path()));
            }
        }
    }

    Ok(None)
}

/// Backup the current account's sdk_data directory
///
/// The backup is stored in `backup_dir/{account_id}/` where `account_id`
/// is derived from the sdk_data directory name.
pub fn backup_account(
    prefix: impl AsRef<Path>,
    backup_dir: impl AsRef<Path>
) -> anyhow::Result<Option<String>> {
    let Some(sdk_data) = get_sdk_data_dir(prefix)? else {
        return Ok(None);
    };

    let account_id = sdk_data
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let target = backup_dir.as_ref().join(&account_id);

    // Remove the old backup and copy the current one
    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }

    copy_dir(&sdk_data, &target)?;

    Ok(Some(account_id))
}

/// Restore a backed up account's sdk_data directory
///
/// Removes the current sdk_data directory and copies the backup in its
/// place.
pub fn restore_account(
    prefix: impl AsRef<Path>,
    backup_dir: impl AsRef<Path>,
    account_id: impl AsRef<str>
) -> anyhow::Result<()> {
    let backup = backup_dir.as_ref().join(account_id.as_ref());

    if !backup.exists() {
        anyhow::bail!("Account backup doesn't exist: {}", backup.display());
    }

    let prefix = prefix.as_ref();

    // Remove the current sdk_data directory
    if let Some(current) = get_sdk_data_dir(prefix)? {
        std::fs::remove_dir_all(&current)?;
    }

    // Restore the backup to the game's app data directory
    let users_dir = prefix.join("drive_c/users");

    let mut target = None;

    for entry in std::fs::read_dir(&users_dir)? {
        let entry = entry?;

        if !entry.file_type()?.is_dir() {
            continue;
        }

        let app_data = entry
            .path()
            .join("AppData/LocalLow/Hypergryph/Arknights");

        if app_data.exists() {
            target = Some(app_data.join(account_id.as_ref()));
            break;
        }
    }

    let Some(target) = target else {
        anyhow::bail!("Game app data directory not found in the prefix");
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    copy_dir(&backup, &target)?;

    Ok(())
}

/// List all backed up accounts
pub fn list_accounts(backup_dir: impl AsRef<Path>) -> anyhow::Result<Vec<String>> {
    let backup_dir = backup_dir.as_ref();

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();

    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            accounts.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    Ok(accounts)
}

/// Recursively copy a directory
fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to)?;

    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir(&source, &target)?;
        }
        else {
            std::fs::copy(&source, &target)?;
        }
    }

    Ok(())
}
