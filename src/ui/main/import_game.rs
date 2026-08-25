use std::path::PathBuf;

use crate::*;
use super::{App, AppMsg};

pub fn import_game(sender: relm4::ComponentSender<App>, path: PathBuf) {
    // reject xdg-document-portal fuse mounts, files appear present but vanish
    // when the portal closes, so the game would silently break
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    for p in [path.as_path(), canonical.as_path()] {
        let s = p.to_string_lossy();
        if s.starts_with("/run/user/") && s.contains("/doc/") {
            sender.input(AppMsg::Toast {
                title: tr!("import-game-path-runtime"),
                description: None
            });
            return;
        }
    }

    let mut config = match Config::get() {
        Ok(c) => c,
        Err(err) => {
            sender.input(AppMsg::Toast {
                title: tr!("import-game-error"),
                description: Some(err.to_string())
            });
            return;
        }
    };

    let edition = config.launcher.edition;
    let game = Game::new(&path, edition);

    if !game.is_installed() {
        sender.input(AppMsg::Toast {
            title: tr!("import-game-invalid-path"),
            description: None
        });
        return;
    }

    // write .version if missing so the launcher can detect the version
    let version_path = path.join(".version");
    if !version_path.exists() {
        match game.get_version() {
            Ok(version) => {
                if let Err(err) = std::fs::write(&version_path, version.to_string()) {
                    tracing::warn!("Failed to write .version during import: {err}");
                }
            }
            Err(err) => tracing::warn!("Failed to detect version during import: {err}")
        }
    }

    match edition {
        GameEdition::Official => config.game.path.official = path,
        GameEdition::Bilibili => config.game.path.bilibili = path
    }
    Config::update(config);

    sender.input(AppMsg::UpdateLauncherState {
        perform_on_download_needed: false,
        show_status_page: true
    });
}
