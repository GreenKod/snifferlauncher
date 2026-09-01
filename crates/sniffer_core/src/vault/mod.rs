//! Native Data Vault Engine
//!
//! Provides a high-performance, thread-safe, and reactive in-memory generic data store with
//! persistence and sandboxed namespace isolation for plugins and system services.

pub mod files;
pub mod models;

pub use models::SystemTheme;

use obfstr::obfstr;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// The Native Data Vault engine managing generic system, plugin, and shared data.
#[derive(Clone)]
pub struct DataVault {
    /// In-memory Key-Value store.
    kv_store: Arc<RwLock<HashMap<String, String>>>,
    /// Cache directory for persisting vault data and files.
    cache_dir: Arc<RwLock<Option<PathBuf>>>,
}

impl Default for DataVault {
    fn default() -> Self {
        Self::new(None)
    }
}

impl DataVault {
    /// Create a new generic DataVault instance.
    #[must_use]
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let vault = Self {
            kv_store: Arc::new(RwLock::new(HashMap::new())),
            cache_dir: Arc::new(RwLock::new(cache_dir)),
        };

        // Initialize default system theme into generic KV store
        let _ = vault.set_json("system.theme", &SystemTheme::default(), "system");

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

    /// Write a Key-Value pair into the vault with sandboxed namespace validation.
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

    /// Helper to serialize and write any JSON-serializable value into the vault.
    pub fn set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        plugin_id: &str,
    ) -> Result<(), String> {
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        self.set(key, &json, plugin_id)
    }

    /// Read a Key-Value pair from the vault with sandboxed namespace validation.
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

    /// Helper to read and deserialize any JSON-serializable value from the vault.
    #[must_use]
    pub fn get_json<T: DeserializeOwned>(&self, key: &str, plugin_id: &str) -> Option<T> {
        let json_str = self.get(key, plugin_id)?;
        serde_json::from_str(&json_str).ok()
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
