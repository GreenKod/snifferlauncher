//! Native Data Vault Engine
//!
//! Provides a high-performance, thread-safe, and reactive in-memory data store with
//! persistence and native fuzzy indexing for launcher applications.

pub mod files;
pub mod models;

pub use models::{AppQueryResult, QueryAppsParams, SystemTheme};

use crate::types::AppInfo;
use obfstr::obfstr;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// The Native Data Vault engine managing system, plugin, and shared data.
#[derive(Clone)]
pub struct DataVault {
    /// Authoritative cache of installed applications in Rust memory.
    system_apps: Arc<RwLock<Vec<AppInfo>>>,
    /// In-memory Key-Value store.
    kv_store: Arc<RwLock<HashMap<String, String>>>,
    /// Cache directory for persisting vault data.
    cache_dir: Arc<RwLock<Option<PathBuf>>>,
}

impl Default for DataVault {
    fn default() -> Self {
        Self::new(None)
    }
}

impl DataVault {
    /// Create a new DataVault instance.
    #[must_use]
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let vault = Self {
            system_apps: Arc::new(RwLock::new(Vec::new())),
            kv_store: Arc::new(RwLock::new(HashMap::new())),
            cache_dir: Arc::new(RwLock::new(cache_dir)),
        };

        // Initialize default system theme
        vault.update_system_theme(SystemTheme::default());

        // Load existing disk-persisted plugin data if cache directory exists
        vault.load_persisted_data();

        vault
    }

    /// Updates or configures the cache directory dynamically.
    pub fn set_cache_dir(&self, dir: PathBuf) {
        if let Ok(mut lock) = self.cache_dir.write() {
            *lock = Some(dir);
        }
        self.load_persisted_data();
    }

    /// Updates the system application cache.
    pub fn update_system_apps(&self, apps: Vec<AppInfo>) {
        if let Ok(json) = serde_json::to_string(&apps) {
            let _ = self.set("system.apps", &json, "system");
        }
        if let Ok(mut lock) = self.system_apps.write() {
            *lock = apps;
        }
    }

    /// Updates the operating system theme in the Data Vault.
    pub fn update_system_theme(&self, theme: SystemTheme) {
        if let Ok(json) = serde_json::to_string(&theme) {
            let _ = self.set("system.theme", &json, "system");
        }
    }

    /// Fast native querying & fuzzy filtering of installed applications.
    #[must_use]
    pub fn query_apps(&self, params: &QueryAppsParams) -> AppQueryResult {
        let apps_guard = match self.system_apps.read() {
            Ok(g) => g,
            Err(_) => {
                return AppQueryResult {
                    apps: Vec::new(),
                    total_count: 0,
                    page: 1,
                    total_pages: 0,
                };
            }
        };

        let mut filtered: Vec<AppInfo> = if let Some(ref query) = params.search {
            let q = query.trim().to_lowercase();
            if q.is_empty() {
                apps_guard.clone()
            } else {
                apps_guard
                    .iter()
                    .filter(|app| {
                        app.name.to_lowercase().contains(&q)
                            || app.package_name.to_lowercase().contains(&q)
                    })
                    .cloned()
                    .collect()
            }
        } else {
            apps_guard.clone()
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

    /// Write a Key-Value pair into the vault.
    pub fn set(&self, key: &str, value: &str, plugin_id: &str) -> Result<(), String> {
        let actual_key = if key.starts_with("system.") {
            if plugin_id != "system" {
                return Err(
                    obfstr!("Permission denied: cannot write to system.* namespace").to_string(),
                );
            }
            key.to_string()
        } else if key.starts_with("shared.") {
            key.to_string()
        } else if key.starts_with("plugin.") {
            let expected_prefix = format!("plugin.{plugin_id}.");
            if !key.starts_with(&expected_prefix) {
                return Err(obfstr!("Permission denied: cross-plugin key mutation").to_string());
            }
            key.to_string()
        } else {
            format!("plugin.{plugin_id}.{key}")
        };

        if let Ok(mut store) = self.kv_store.write() {
            store.insert(actual_key.clone(), value.to_string());
        }

        if actual_key.starts_with("plugin.") {
            self.persist_plugin_data(plugin_id);
        }

        Ok(())
    }

    /// Read a Key-Value pair from the vault.
    #[must_use]
    pub fn get(&self, key: &str, plugin_id: &str) -> Option<String> {
        let store = self.kv_store.read().ok()?;

        if key.starts_with("system.") || key.starts_with("shared.") {
            return store.get(key).cloned();
        }

        let full_key = if key.starts_with("plugin.") {
            let expected_prefix = format!("plugin.{plugin_id}.");
            if !key.starts_with(&expected_prefix) {
                return None;
            }
            key.to_string()
        } else {
            format!("plugin.{plugin_id}.{key}")
        };

        store.get(&full_key).cloned()
    }

    /// Delete a Key-Value pair from the vault.
    pub fn delete(&self, key: &str, plugin_id: &str) -> Result<bool, String> {
        let actual_key = if key.starts_with("system.") {
            if plugin_id != "system" {
                return Err(
                    obfstr!("Permission denied: cannot delete from system.* namespace").to_string(),
                );
            }
            key.to_string()
        } else if key.starts_with("shared.") {
            key.to_string()
        } else if key.starts_with("plugin.") {
            let expected_prefix = format!("plugin.{plugin_id}.");
            if !key.starts_with(&expected_prefix) {
                return Err(obfstr!("Permission denied: cross-plugin key deletion").to_string());
            }
            key.to_string()
        } else {
            format!("plugin.{plugin_id}.{key}")
        };

        let removed = if let Ok(mut store) = self.kv_store.write() {
            store.remove(&actual_key).is_some()
        } else {
            false
        };

        if removed && key.starts_with("plugin.") {
            self.persist_plugin_data(plugin_id);
        }

        Ok(removed)
    }

    /// List keys matching an optional prefix.
    #[must_use]
    pub fn keys(&self, prefix: Option<&str>, plugin_id: &str) -> Vec<String> {
        let store = match self.kv_store.read() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        let my_plugin_prefix = format!("plugin.{plugin_id}.");

        store
            .keys()
            .filter(|k| {
                if k.starts_with("plugin.") && !k.starts_with(&my_plugin_prefix) {
                    return false;
                }
                if let Some(p) = prefix {
                    k.starts_with(p)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }
}
