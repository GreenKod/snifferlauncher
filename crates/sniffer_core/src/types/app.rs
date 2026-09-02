use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64};

pub static SCREEN_WIDTH: AtomicU32 = AtomicU32::new(0);
pub static SCREEN_HEIGHT: AtomicU32 = AtomicU32::new(0);
pub static UI_VERSION: AtomicU64 = AtomicU64::new(0);

/// Metadata about an installed application.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

/// Parameters for querying, filtering, and paginating application lists.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct QueryAppsParams {
    pub search: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

/// Result of querying installed applications with pagination metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppQueryResult {
    pub apps: Vec<AppInfo>,
    pub total_count: usize,
    pub page: usize,
    pub total_pages: usize,
}

/// Fast native querying & fuzzy filtering of an application list.
#[must_use]
pub fn query_apps(apps: &[AppInfo], params: &QueryAppsParams) -> AppQueryResult {
    let mut filtered: Vec<AppInfo> = if let Some(ref query) = params.search {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            apps.to_vec()
        } else {
            apps.iter()
                .filter(|app| {
                    app.name.to_lowercase().contains(&q)
                        || app.package_name.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        }
    } else {
        apps.to_vec()
    };

    filtered.sort_by_key(|a| a.name.to_lowercase());

    let total_count = filtered.len();
    let limit = params.limit.unwrap_or(28).max(1);
    let total_pages = if total_count == 0 {
        1
    } else {
        total_count.div_ceil(limit)
    };
    let page = params.page.unwrap_or(0).min(total_pages.saturating_sub(1));

    let start = page * limit;
    let paged_apps = if start < total_count {
        let end = (start + limit).min(total_count);
        filtered[start..end].to_vec()
    } else {
        Vec::new()
    };

    AppQueryResult {
        apps: paged_apps,
        total_count,
        page,
        total_pages,
    }
}

/// High-level action dispatched by the host or plugins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    LaunchApp { package_name: String },
    LoadImage { id: String, src: String },
    FocusTextInput(String),
    BlurTextInput,
    RequestDefaultLauncher,
}

/// A sample state structure used for binary `postcard` / `ArrayBuffer` state passing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub click_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
}
