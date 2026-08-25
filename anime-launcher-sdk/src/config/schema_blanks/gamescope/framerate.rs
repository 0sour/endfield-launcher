use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamescopeFramerate {
    /// Focused framerate limit.
    ///
    /// ```text
    /// --nested-refresh
    /// ```
    pub focused: Option<u64>,

    /// Unfocused framerate limit.
    ///
    /// ```text
    /// --nested-unfocused-refresh
    /// ```
    pub unfocused: Option<u64>
}

impl GamescopeFramerate {
    #[inline]
    pub fn get_command(&self) -> String {
        let mut flags = Vec::with_capacity(2);

        if let Some(focused) = &self.focused {
            flags.push(format!("--nested-refresh {focused}"));
        }

        if let Some(unfocused) = &self.unfocused {
            flags.push(format!("--nested-unfocused-refresh {unfocused}"));
        }

        flags.join(" ")
    }
}

impl From<&JsonValue> for GamescopeFramerate {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            focused: value.get("focused")
                .and_then(|value| {
                    if value.is_null() {
                        Some(None)
                    } else {
                        value.as_u64().map(Some)
                    }
                })
                .unwrap_or(default.focused),

            unfocused: value.get("unfocused")
                .and_then(|value| {
                    if value.is_null() {
                        Some(None)
                    } else {
                        value.as_u64().map(Some)
                    }
                })
                .unwrap_or(default.unfocused)
        }
    }
}
