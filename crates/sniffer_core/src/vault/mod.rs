//! Native Data Vault Engine
//!
//! Provides a high-performance, thread-safe, and reactive in-memory data store with
//! persistence and native fuzzy indexing for launcher applications.

use crate::dev_log;
use crate::types::AppInfo;
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

/// Represents the active operating system theme colors and mode.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

        // If cache directory is set, load any existing snapshots
        vault.load_persisted_data();
        vault
    }

    /// Set or update the system theme in the vault under "system.theme".
    pub fn update_system_theme(&self, theme: SystemTheme) {
        if let Ok(json) = serde_json::to_string(&theme) {
            let _ = self.set("system.theme", json, "system");
        }
    }

    /// Set or update the cache directory for persistence and file vaults.
    pub fn set_cache_dir(&self, dir: PathBuf) {
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut guard) = self.cache_dir.write() {
            *guard = Some(dir);
        }
        self.load_persisted_data();
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
            total_count.div_ceil(limit)
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
            let count = self.system_apps.read().map_or(0, |g| g.len());
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

    /// Save a binary file (e.g. image, asset) to the plugin's isolated file vault.
    ///
    /// # Errors
    /// Returns error if file_name contains illegal path traversal characters or disk write fails.
    pub fn save_file(
        &self,
        file_name: &str,
        data: &[u8],
        plugin_id: &str,
    ) -> Result<String, String> {
        let clean_name = std::path::Path::new(file_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| obfstr!("Invalid file name").to_string())?;

        let cache_dir_guard = self
            .cache_dir
            .read()
            .map_err(|_| obfstr!("Cache lock error").to_string())?;
        let cache_dir = cache_dir_guard
            .as_ref()
            .ok_or_else(|| obfstr!("No cache directory configured for DataVault").to_string())?;

        let files_dir = cache_dir.join(format!("vault_{plugin_id}")).join("files");
        if !files_dir.exists() {
            let _ = std::fs::create_dir_all(&files_dir);
        }

        let target_file = files_dir.join(clean_name);
        std::fs::write(&target_file, data)
            .map_err(|e| format!("{}: {e}", obfstr!("Failed to write file")))?;

        dev_log!(
            "{} '{}' {} '{}'",
            obfstr!("[DataVault] Saved file"),
            clean_name,
            obfstr!("for plugin"),
            plugin_id
        );

        Ok(format!("vault://{clean_name}"))
    }

    /// Read a binary file from the plugin's isolated file vault.
    #[must_use]
    pub fn read_file(&self, file_name: &str, plugin_id: &str) -> Option<Vec<u8>> {
        let clean_name = std::path::Path::new(file_name)
            .file_name()
            .and_then(|n| n.to_str())?;

        let cache_dir_guard = self.cache_dir.read().ok()?;
        let cache_dir = cache_dir_guard.as_ref()?;
        let target_file = cache_dir
            .join(format!("vault_{plugin_id}"))
            .join("files")
            .join(clean_name);

        if target_file.exists() {
            std::fs::read(&target_file).ok()
        } else {
            None
        }
    }

    /// Delete a file from the plugin's isolated file vault.
    pub fn delete_file(&self, file_name: &str, plugin_id: &str) -> Result<bool, String> {
        let clean_name = std::path::Path::new(file_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| obfstr!("Invalid file name").to_string())?;

        let cache_dir_guard = self
            .cache_dir
            .read()
            .map_err(|_| obfstr!("Cache lock error").to_string())?;
        let cache_dir = cache_dir_guard
            .as_ref()
            .ok_or_else(|| obfstr!("No cache directory configured for DataVault").to_string())?;

        let target_file = cache_dir
            .join(format!("vault_{plugin_id}"))
            .join("files")
            .join(clean_name);

        if target_file.exists() {
            std::fs::remove_file(&target_file)
                .map(|_| true)
                .map_err(|e| format!("{}: {e}", obfstr!("Failed to delete file")))
        } else {
            Ok(false)
        }
    }

    /// List all files saved in the plugin's isolated file vault.
    #[must_use]
    pub fn list_files(&self, plugin_id: &str) -> Vec<String> {
        let cache_dir_guard = match self.cache_dir.read() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let cache_dir = match cache_dir_guard.as_ref() {
            Some(dir) => dir,
            None => return Vec::new(),
        };

        let files_dir = cache_dir.join(format!("vault_{plugin_id}")).join("files");
        if !files_dir.exists() {
            return Vec::new();
        }

        if let Ok(entries) = std::fs::read_dir(files_dir) {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Resolves a `vault://<filename>` URI into an absolute filesystem path for rendering.
    #[must_use]
    pub fn resolve_vault_file_path(&self, uri: &str, plugin_id: &str) -> Option<PathBuf> {
        let file_name = uri.strip_prefix("vault://")?;
        let clean_name = std::path::Path::new(file_name).file_name()?.to_str()?;
        let cache_dir_guard = self.cache_dir.read().ok()?;
        let cache_dir = cache_dir_guard.as_ref()?;
        let target_file = cache_dir
            .join(format!("vault_{plugin_id}"))
            .join("files")
            .join(clean_name);

        if target_file.exists() {
            Some(target_file)
        } else {
            None
        }
    }

    /// Save plugin KV pairs to disk.
    fn persist_plugin_data(&self, plugin_id: &str) {
        let cache_dir_guard = match self.cache_dir.read() {
            Ok(g) => g,
            Err(_) => return,
        };
        let cache_dir = match cache_dir_guard.as_ref() {
            Some(dir) => dir,
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
        let cache_dir_guard = match self.cache_dir.read() {
            Ok(g) => g,
            Err(_) => return,
        };
        let cache_dir = match cache_dir_guard.as_ref() {
            Some(dir) => dir,
            None => return,
        };

        if !cache_dir.exists() {
            let _ = std::fs::create_dir_all(cache_dir);
            return;
        }

        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str())
                    && file_name.starts_with("vault_")
                    && file_name.ends_with(".json")
                    && let Ok(content) = std::fs::read_to_string(&path)
                    && let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content)
                    && let Ok(mut store) = self.kv_store.write()
                {
                    for (k, v) in map {
                        store.insert(k, v);
                    }
                    dev_log!(
                        "{} '{}'",
                        obfstr!("[DataVault] Loaded persistent vault for"),
                        file_name
                    );
                }
            }
        }
    }
}
