use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use anime_game_core::endfield::consts::GameEdition;

use crate::endfield::consts::launcher_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paths {
    pub official: PathBuf,
    pub bilibili: PathBuf
}

impl Paths {
    #[inline]
    /// Get game path for given edition
    pub fn for_edition(&self, edition: impl Into<GameEdition>) -> &Path {
        match edition.into() {
            GameEdition::Official => self.official.as_path(),
            GameEdition::Bilibili => self.bilibili.as_path()
        }
    }
}

impl Default for Paths {
    fn default() -> Self {
        let launcher_dir = launcher_dir().expect("Failed to get launcher dir");

        Self {
            official: launcher_dir.join(concat!("End", "field")),
            bilibili: launcher_dir.join(concat!("End", "field"))
        }
    }
}

impl From<&JsonValue> for Paths {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            official: value.get("official")
                .and_then(JsonValue::as_str)
                .map(PathBuf::from)
                .unwrap_or(default.official),

            bilibili: value.get("bilibili")
                .and_then(JsonValue::as_str)
                .map(PathBuf::from)
                .unwrap_or(default.bilibili),
        }
    }
}
