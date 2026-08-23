use super::DataVault;
use crate::dev_log;
use obfstr::obfstr;
use std::collections::HashMap;
use std::path::PathBuf;

impl DataVault {
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
    pub(crate) fn persist_plugin_data(&self, plugin_id: &str) {
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
    pub(crate) fn load_persisted_data(&self) {
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
