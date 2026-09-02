use serde::{Deserialize, Serialize};

// Re-export application query types from `crate::types` for backwards compatibility
pub use crate::types::{AppQueryResult, QueryAppsParams};

/// Represents the active operating system theme colors and mode.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemTheme {
    pub is_dark: bool,
    pub mode: String, // "dark" | "light"
    pub accent_color: String,
    pub bg_color: String,
    pub text_color: String,
    pub card_bg: String,
}

impl Default for SystemTheme {
    fn default() -> Self {
        Self {
            is_dark: true,
            mode: "dark".to_string(),
            accent_color: "#38BDF8".to_string(),
            bg_color: "#0F172A".to_string(),
            text_color: "#F8FAFC".to_string(),
            card_bg: "#1E293B".to_string(),
        }
    }
}

impl SystemTheme {
    #[must_use]
    pub fn dark() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn light() -> Self {
        Self {
            is_dark: false,
            mode: "light".to_string(),
            accent_color: "#0284C7".to_string(),
            bg_color: "#F8FAFC".to_string(),
            text_color: "#0F172A".to_string(),
            card_bg: "#FFFFFF".to_string(),
        }
    }
}
