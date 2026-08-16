//! Native Data Vault Engine
//!
//! Provides a high-performance, thread-safe, and reactive in-memory data store with
//! persistence and native fuzzy indexing for launcher applications.

use crate::core::types::AppInfo;
use crate::dev_log;
use obfstr::obfstr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppQueryResult {
    pub apps: Vec<AppInfo>,
    pub total_count: usize,
    pub page: usize,
    pub total_pages: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct QueryAppsParams {
    pub search: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

/// The Native Data Vault engine managing system, plugin, and shared data.
#[derive(Clone)]
pub struct DataVault {
    /// Authoritative cache of installed applications in Rust memory.
    system_apps: Arc<RwLock<Vec<AppInfo>>>,
    /// In-memory Key-Value store.
    kv_store: Arc<RwLock<HashMap<String, String>>>,
    /// Cache directory for persisting vault data.
    cache_dir: Option<PathBuf>,
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
            cache_dir,
        };

        // If cache directory is set, load any existing snapshots
        vault.load_persisted_data();
        vault
    }

    /// Set or update the installed applications list in the vault.
    pub fn update_system_apps(&self, apps: Vec<AppInfo>) {
        if let Ok(mut guard) = self.system_apps.write() {
            *guard = apps;
        }
    }

    /// Get all installed applications.
    #[must_use]
    pub fn get_system_apps(&self) -> Vec<AppInfo> {
        self.system_apps
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Fast native query and fuzzy search for applications with pagination.
    #[must_use]
    pub fn query_apps(&self, params: QueryAppsParams) -> AppQueryResult {
        let apps_guard = self.system_apps.read();
        let all_apps = match apps_guard {
            Ok(ref guard) => guard.as_slice(),
            Err(_) => &[],
        };

        let search_term = params.search.as_deref().map(str::to_lowercase);
        let filtered: Vec<AppInfo> = match search_term {
            Some(ref query) if !query.trim().is_empty() => {
                let q = query.trim();
                all_apps
                    .iter()
                    .filter(|app| {
                        app.name.to_lowercase().contains(q)
                            || app.package_name.to_lowercase().contains(q)
                    })
                    .cloned()
                    .collect()
            }
            _ => all_apps.to_vec(),
        };

        let total_count = filtered.len();
        let limit = params.limit.unwrap_or(28).max(1);
        let total_pages = if total_count == 0 {
            1
        } else {
            (total_count + limit - 1) / limit
        };

        let page = params.page.unwrap_or(0).min(total_pages.saturating_sub(1));
        let start = page * limit;
        let end = (start + limit).min(total_count);

        let paged_apps = if start < total_count {
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

    /// Read a value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        // Special virtual keys
        if key == "system.apps" {
            let apps = self.get_system_apps();
            return serde_json::to_string(&apps).ok();
        }
        if key == "system.app_count" {
            let count = self
                .system_apps
                .read()
                .map(|g| g.len())
                .unwrap_or(0);
            return Some(count.to_string());
        }

        self.kv_store.read().ok()?.get(key).cloned()
    }

    /// Write a value to the vault with namespace validation.
    ///
    /// # Errors
    /// Returns error if attempting to write to read-only `system.*` namespace or
    /// another plugin's private `plugin.<other_id>.*` namespace.
    pub fn set(&self, key: &str, value: String, plugin_id: &str) -> Result<(), String> {
        // Namespace security validation
        if key.starts_with("system.") {
            return Err(obfstr!("'system.*' namespace is read-only for plugins").to_string());
        }

        let prefix = format!("plugin.{plugin_id}.");
        if key.starts_with("plugin.") && !key.starts_with(&prefix) {
            return Err(format!(
                "{} '{key}'",
                obfstr!("Plugin is not authorized to write to private namespace")
            ));
        }

        if let Ok(mut store) = self.kv_store.write() {
            store.insert(key.to_string(), value);
        }

        // Auto-save persistent plugin data
        if key.starts_with("plugin.") {
            self.persist_plugin_data(plugin_id);
        }

        Ok(())
    }

    /// Delete a key from the vault.
    pub fn delete(&self, key: &str, plugin_id: &str) -> Result<bool, String> {
        if key.starts_with("system.") {
            return Err(obfstr!("Cannot delete from 'system.*' namespace").to_string());
        }

        let prefix = format!("plugin.{plugin_id}.");
        if key.starts_with("plugin.") && !key.starts_with(&prefix) {
            return Err(obfstr!("Cannot delete from another plugin's namespace").to_string());
        }

        let removed = if let Ok(mut store) = self.kv_store.write() {
            store.remove(key).is_some()
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
                // Security check: plugins can only list system.*, shared.*, and their own plugin.<id>.*
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

    /// Save plugin KV pairs to disk.
    fn persist_plugin_data(&self, plugin_id: &str) {
        let cache_dir = match self.cache_dir {
            Some(ref dir) => dir,
            None => return,
        };

        let prefix = format!("plugin.{plugin_id}.");
        let plugin_data: HashMap<String, String> = if let Ok(store) = self.kv_store.read() {
            store
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            return;
        };

        let file_path = cache_dir.join(format!("vault_{plugin_id}.json"));
        if let Ok(json) = serde_json::to_string(&plugin_data) {
            let _ = std::fs::write(file_path, json);
        }
    }

    /// Load persisted plugin vaults from disk cache.
    fn load_persisted_data(&self) {
        let cache_dir = match self.cache_dir {
            Some(ref dir) => dir,
            None => return,
        };

        if !cache_dir.exists() {
            let _ = std::fs::create_dir_all(cache_dir);
            return;
        }

        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.starts_with("vault_") && file_name.ends_with(".json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                                if let Ok(mut store) = self.kv_store.write() {
                                    for (k, v) in map {
                                        store.insert(k, v);
                                    }
                                }
                                dev_log!("{} '{}'", obfstr!("[DataVault] Loaded persistent vault for"), file_name);
                            }
                        }
                    }
                }
            }
        }
    }
}
