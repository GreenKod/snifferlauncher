use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64};

pub static SCREEN_WIDTH: AtomicU32 = AtomicU32::new(0);
pub static SCREEN_HEIGHT: AtomicU32 = AtomicU32::new(0);
pub static UI_VERSION: AtomicU64 = AtomicU64::new(0);

/// Metadata about an installed application.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub package_name: String,
}

impl AppInfo {
    #[must_use]
    pub const fn new(name: String, package_name: String) -> Self {
        Self { name, package_name }
    }

    /// Creates an `AppInfo` instance with only basic information.
    #[must_use]
    pub const fn from_package_info(name: String, package_name: String) -> Self {
        Self { name, package_name }
    }

    /// Filters apps by name for the launcher search bar.
    pub fn search_by_name<Q: AsRef<str>>(apps: &[Self], query: Q) -> Vec<&Self> {
        let query_str = query.as_ref().to_lowercase();
        if query_str.is_empty() {
            return apps.iter().collect();
        }

        apps.iter()
            .filter(|app| app.name.to_lowercase().contains(&query_str))
            .collect()
    }
}

/// High-level action dispatched by the host or plugins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    OpenSettings,
    OpenContacts,
    OpenCamera,
    LaunchApp { package_name: String },
    LoadImage { id: String, src: String },
    FocusTextInput(String),
    BlurTextInput,
    RequestDefaultLauncher,
}

/// A sample state structure to demonstrate `Bincode` / `ArrayBuffer` passing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub click_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
}

/// Core application trait — implemented by platform-agnostic app logic.
pub trait Application {
    type Message: Clone + std::fmt::Debug;
    type State: Default;

    fn update(state: &mut Self::State, msg: Self::Message) -> Option<Action>;

    /// Build the UI element tree for the current state and screen metrics.
    ///
    /// `metrics` carries the DPI scale factor and logical screen dimensions so
    /// the view can adapt padding, font sizes, and element heights to the device.
    fn view(state: &Self::State, metrics: &crate::ScreenMetrics) -> super::element::Element;
}
