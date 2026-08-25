use serde::{Deserialize, Serialize};

/// Game edition (server channel) for Arknights CN
///
/// The CN client has two channels: the official one (Hypergryph) and the
/// Bilibili one. They share the same game files but use different SDK DLLs
/// and channel parameters in the launcher API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameEdition {
    /// Official channel (channel=1, sub_channel=1)
    Official,

    /// Bilibili channel (channel=2, sub_channel=2)
    Bilibili
}

impl Default for GameEdition {
    #[inline]
    fn default() -> Self {
        Self::Official
    }
}

impl GameEdition {
    #[inline]
    pub fn list() -> &'static [GameEdition] {
        &[Self::Official, Self::Bilibili]
    }

    /// Launcher API URL (batch_proxy)
    #[inline]
    pub fn api_uri(&self) -> &str {
        concat!(
            "https://launcher.",
            "hypergryph",
            ".com/api/proxy/batch_proxy"
        )
    }

    /// Web launcher API URL (banners, announcements, background images)
    #[inline]
    pub fn web_api_uri(&self) -> &str {
        concat!(
            "https://launcher.",
            "hypergryph",
            ".com/api/proxy/web/batch_proxy"
        )
    }

    /// Game appcode
    #[inline]
    pub fn app_code(&self) -> &str {
        "GzD1CpaWgmSq1wew"
    }

    /// Launcher appcode (shared between all CN games)
    #[inline]
    pub fn launcher_app_code(&self) -> &str {
        "abYeZZ16BPluCFyT"
    }

    /// Channel parameter
    #[inline]
    pub fn channel(&self) -> &str {
        match self {
            Self::Official => "1",
            Self::Bilibili => "2"
        }
    }

    /// Sub-channel parameter
    #[inline]
    pub fn sub_channel(&self) -> &str {
        match self {
            Self::Official => "1",
            Self::Bilibili => "2"
        }
    }

    /// Request sequence number
    #[inline]
    pub fn seq(&self) -> &str {
        "5"
    }

    /// Game data folder name
    #[inline]
    pub fn data_folder(&self) -> &str {
        concat!("Ark", "nights", "_Data")
    }

    /// Game executable name
    #[inline]
    pub fn exe_name(&self) -> &str {
        "Arknights.exe"
    }

    pub fn from_system_lang() -> Self {
        let locale = std::env::var("LC_ALL")
            .unwrap_or_else(|_| {
                std::env::var("LC_MESSAGES")
                    .unwrap_or_else(|_| std::env::var("LANG").unwrap_or(String::from("en_us")))
            })
            .to_ascii_lowercase();

        if locale.starts_with("zh_cn") {
            Self::Official
        }
        else {
            Self::Official
        }
    }
}
