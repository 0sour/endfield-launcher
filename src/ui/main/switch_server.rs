use gtk::prelude::*;
use adw::prelude::*;

use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::endfield::config::Config;
use anime_launcher_sdk::endfield::account;
use anime_launcher_sdk::anime_game_core::endfield::consts::GameEdition;
use anime_launcher_sdk::anime_game_core::endfield::payload;

use crate::*;

#[derive(Debug)]
pub struct SwitchServerDialog {
    visible: bool,
    switching: bool,
    accounts: Vec<String>
}

#[derive(Debug)]
pub enum SwitchServerDialogMsg {
    Show,
    Hide,
    SwitchServer,
    BackupAccount,
    RestoreAccount(String),
    SetAccounts(Vec<String>)
}

#[relm4::component(pub)]
impl SimpleComponent for SwitchServerDialog {
    type Init = ();
    type Input = SwitchServerDialogMsg;
    type Output = ();

    view! {
        dialog = adw::Window {
            set_title: Some(&tr!("switch-server")),
            set_default_size: (420, 420),
            set_modal: true,

            set_visible: model.visible,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 18,

                adw::ComboRow {
                    set_title: &tr!("server"),
                    set_subtitle: &tr!("server-description"),

                    set_model: Some(&gtk::StringList::new(&[
                        "官服",
                        "B服"
                    ])),

                    set_selected: match CONFIG.launcher.edition {
                        GameEdition::Official => 0,
                        GameEdition::Bilibili => 1
                    }
                },

                gtk::Button {
                    set_label: &tr!("switch-server-action"),
                    set_css_classes: &["suggested-action", "pill"],

                    #[watch]
                    set_sensitive: !model.switching,

                    connect_clicked => SwitchServerDialogMsg::SwitchServer
                },

                gtk::Separator {},

                gtk::Label {
                    set_label: &tr!("accounts"),
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["heading"]
                },

                gtk::Button {
                    set_label: &tr!("backup-account"),
                    connect_clicked => SwitchServerDialogMsg::BackupAccount
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,

                    #[name = "accounts_list"]
                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::Single
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>
    ) -> ComponentParts<Self> {
        tracing::info!("Initializing switch server dialog");

        let accounts = account::list_accounts(
            CONFIG.game.wine.prefix.join("account-backups")
        ).unwrap_or_default();

        let model = SwitchServerDialog {
            visible: false,
            switching: false,
            accounts
        };

        let widgets = view_output!();

        // Populate the accounts list
        for account_id in &model.accounts {
            let row = gtk::ListBoxRow::new();
            let label = gtk::Label::new(Some(account_id));
            label.set_halign(gtk::Align::Start);
            row.set_child(Some(&label));
            widgets.accounts_list.append(&row);
        }

        // Restore the account when a row is selected
        widgets.accounts_list.connect_row_selected(|_, row| {
            if let Some(row) = row {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk::Label>() {
                        let account_id = label.text().to_string();

                        std::thread::spawn(move || {
                            if let Err(err) = restore_account(&account_id) {
                                tracing::error!("Failed to restore account: {err}");
                            }
                        });
                    }
                }
            }
        });

        ComponentParts {
            model,
            widgets
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SwitchServerDialogMsg::Show => {
                self.visible = true;
            }

            SwitchServerDialogMsg::Hide => {
                self.visible = false;
            }

            SwitchServerDialogMsg::SwitchServer => {
                self.switching = true;

                let sender = _sender.clone();

                std::thread::spawn(move || {
                    let result = switch_server();

                    sender.input(SwitchServerDialogMsg::Hide);

                    if let Err(err) = result {
                        tracing::error!("Failed to switch server: {err}");
                    }
                });
            }

            SwitchServerDialogMsg::BackupAccount => {
                let sender = _sender.clone();

                std::thread::spawn(move || {
                    let result = backup_current_account();

                    if let Err(err) = result {
                        tracing::error!("Failed to backup account: {err}");
                    }
                    else {
                        let accounts = account::list_accounts(
                            CONFIG.game.wine.prefix.join("account-backups")
                        ).unwrap_or_default();

                        sender.input(SwitchServerDialogMsg::SetAccounts(accounts));
                    }
                });
            }

            SwitchServerDialogMsg::RestoreAccount(account_id) => {
                let sender = _sender.clone();

                std::thread::spawn(move || {
                    let result = restore_account(&account_id);

                    if let Err(err) = result {
                        tracing::error!("Failed to restore account: {err}");
                    }
                });
            }

            SwitchServerDialogMsg::SetAccounts(accounts) => {
                self.accounts = accounts;
            }
        }
    }
}

/// Switch the game server channel (official <-> bilibili)
///
/// Downloads the channel-specific SDK files from the CDN and deploys
/// them to the game root directory.
fn switch_server() -> anyhow::Result<()> {
    let mut config = Config::get()?;

    let current = config.launcher.edition;
    let target = match current {
        GameEdition::Official => GameEdition::Bilibili,
        GameEdition::Bilibili => GameEdition::Official
    };

    let game_path = config.game.path.for_edition(target).to_path_buf();

    if !game_path.exists() {
        anyhow::bail!("Game is not installed");
    }

    tracing::info!("Switching server: {current:?} -> {target:?}");

    // Download the payload files to a temp folder
    let payload_dir = std::env::temp_dir().join(format!("endfield-payload-{target:?}"));

    if payload_dir.exists() {
        std::fs::remove_dir_all(&payload_dir)?;
    }

    payload::download_payload(target, &payload_dir, |done, total| {
        tracing::debug!("Downloading payload: {done}/{total}");
    })?;

    // Deploy the payload files to the game root
    payload::deploy_payload(&payload_dir, &game_path, |done, total| {
        tracing::debug!("Deploying payload: {done}/{total}");
    })?;

    // Clean up the temp folder
    std::fs::remove_dir_all(&payload_dir)?;

    // Update the config
    config.launcher.edition = target;
    Config::update(config);

    tracing::info!("Server switched to {target:?}");

    Ok(())
}

/// Backup the current account's sdk_data directory
fn backup_current_account() -> anyhow::Result<()> {
    let config = Config::get()?;

    let backup_dir = config.game.wine.prefix.join("account-backups");

    match account::backup_account(&config.game.wine.prefix, &backup_dir)? {
        Some(account_id) => {
            tracing::info!("Account backed up: {account_id}");
            Ok(())
        }
        None => {
            tracing::warn!("No sdk_data directory found, nothing to backup");
            Ok(())
        }
    }
}

/// Restore a backed up account
fn restore_account(account_id: &str) -> anyhow::Result<()> {
    let config = Config::get()?;

    let backup_dir = config.game.wine.prefix.join("account-backups");

    account::restore_account(&config.game.wine.prefix, &backup_dir, account_id)?;

    tracing::info!("Account restored: {account_id}");

    Ok(())
}
