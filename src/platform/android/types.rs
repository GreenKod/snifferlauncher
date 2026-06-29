/// Metadata about an installed Android application.
#[derive(Debug)]
pub struct AppInfo {
    pub name: String,
    pub package_name: String,
    pub icon_bytes: Vec<u8>,
}

impl AppInfo {
    #[must_use]
    pub const fn new(name: String, package_name: String, icon_bytes: Vec<u8>) -> Self {
        Self {
            name,
            package_name,
            icon_bytes,
        }
    }

    /// Creates an `AppInfo` instance with only basic information and no icon.
    /// Icon data can be added later using `with_icon`.
    #[must_use]
    pub const fn from_package_info(name: String, package_name: String) -> Self {
        Self {
            name,
            package_name,
            icon_bytes: Vec::new(),
        }
    }

    /// Adds or updates the app icon bytes using a builder-style pattern.
    #[must_use]
    pub fn with_icon(mut self, icon_bytes: Vec<u8>) -> Self {
        self.icon_bytes = icon_bytes;
        self
    }

    /// Filters apps by name for the launcher search bar.
    /// `Q: AsRef<str>` allows both `&str` and `String` to be passed directly.
    pub fn search_by_name<Q: AsRef<str>>(apps: &[Self], query: Q) -> Vec<&Self> {
        let query_str = query.as_ref().to_lowercase();
        if query_str.is_empty() {
            return apps.iter().collect(); // Return all apps when the query is empty
        }

        apps.iter()
            .filter(|app| app.name.to_lowercase().contains(&query_str))
            .collect()
    }
}
