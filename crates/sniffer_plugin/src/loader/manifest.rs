use obfstr::obfstr;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PluginsConfig {
    #[serde(default)]
    pub master_plugin: Option<String>,
    pub active_plugins: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub main: String,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default, rename = "isMaster")]
    pub is_master: bool,
    #[serde(default)]
    pub preload: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default, rename = "defaultSettings")]
    pub default_settings: serde_json::Value,
}

impl PluginManifest {
    /// Validate a manifest and return a list of human-readable issues.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if self.id.trim().is_empty() {
            issues.push(obfstr!("manifest id must not be empty").to_string());
        }
        if self.name.trim().is_empty() {
            issues.push(obfstr!("manifest name must not be empty").to_string());
        }
        if self.version.trim().is_empty() {
            issues.push(obfstr!("manifest version must not be empty").to_string());
        }
        if self.main.trim().is_empty() {
            issues.push(obfstr!("manifest main entry must not be empty").to_string());
        }
        if self.permissions.iter().any(|p| p.trim().is_empty()) {
            issues.push(obfstr!("manifest permissions must not contain empty values").to_string());
        }

        issues
    }
}
