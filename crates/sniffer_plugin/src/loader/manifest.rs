use obfstr::obfstr;
use serde::Deserialize;
use std::collections::HashSet;

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

        let trimmed_id = self.id.trim();
        if trimmed_id.is_empty() {
            issues.push(obfstr!("manifest id must not be empty").to_string());
        } else if !is_valid_plugin_id(trimmed_id) {
            issues.push(format!(
                "{}: '{trimmed_id}' (expected reverse-DNS format like 'com.example.plugin')",
                obfstr!("manifest id has invalid format")
            ));
        }

        let trimmed_name = self.name.trim();
        if trimmed_name.is_empty() {
            issues.push(obfstr!("manifest name must not be empty").to_string());
        } else if trimmed_name.len() > 128 {
            issues.push(obfstr!("manifest name exceeds maximum allowed length of 128 chars").to_string());
        }

        let trimmed_version = self.version.trim();
        if trimmed_version.is_empty() {
            issues.push(obfstr!("manifest version must not be empty").to_string());
        } else if !is_valid_semver(trimmed_version) {
            issues.push(format!(
                "{}: '{trimmed_version}' (expected SemVer, e.g. 1.0.0)",
                obfstr!("manifest version has invalid format")
            ));
        }

        let trimmed_main = self.main.trim();
        if trimmed_main.is_empty() {
            issues.push(obfstr!("manifest main entry must not be empty").to_string());
        } else if let Err(err) = validate_script_path(trimmed_main) {
            issues.push(format!("{}: {err}", obfstr!("manifest main entry path invalid")));
        }

        for (idx, script) in self.scripts.iter().enumerate() {
            let trimmed = script.trim();
            if let Err(err) = validate_script_path(trimmed) {
                issues.push(format!(
                    "{} [#{idx} '{trimmed}']: {err}",
                    obfstr!("manifest script path invalid")
                ));
            }
        }

        for (idx, preload) in self.preload.iter().enumerate() {
            let trimmed = preload.trim();
            if let Err(err) = validate_preload_path(trimmed) {
                issues.push(format!(
                    "{} [#{idx} '{trimmed}']: {err}",
                    obfstr!("manifest preload path invalid")
                ));
            }
        }

        let mut seen_permissions = HashSet::new();
        for (idx, perm) in self.permissions.iter().enumerate() {
            let trimmed = perm.trim();
            if trimmed.is_empty() {
                issues.push(format!(
                    "{} [#{idx}]",
                    obfstr!("manifest permissions must not contain empty values")
                ));
            } else {
                if !seen_permissions.insert(trimmed.to_string()) {
                    issues.push(format!(
                        "{}: '{trimmed}'",
                        obfstr!("duplicate permission declared in manifest")
                    ));
                }
                if !is_valid_permission_name(trimmed) {
                    issues.push(format!(
                        "{}: '{trimmed}' (must follow namespaced format e.g. 'plugin.permission.UI')",
                        obfstr!("permission does not match valid namespace format")
                    ));
                }
            }
        }

        if !self.default_settings.is_null() && !self.default_settings.is_object() {
            issues.push(obfstr!("manifest defaultSettings must be a JSON object if present").to_string());
        }

        issues
    }
}

fn is_valid_plugin_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    if !id.contains('.') {
        return false;
    }
    if id.starts_with('.') || id.ends_with('.') || id.starts_with('-') || id.ends_with('-') {
        return false;
    }
    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2 {
        return false;
    }
    for seg in segments {
        if seg.is_empty() {
            return false;
        }
        let Some(first) = seg.chars().next() else {
            return false;
        };
        if !first.is_ascii_alphanumeric() {
            return false;
        }
        if !seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return false;
        }
    }
    true
}

fn is_valid_semver(version: &str) -> bool {
    if version.is_empty() || version.len() > 32 {
        return false;
    }
    let main_part = version.split(&['-', '+'][..]).next().unwrap_or(version);
    let parts: Vec<&str> = main_part.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    for part in parts {
        if part.is_empty() {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if part.parse::<u64>().is_err() {
            return false;
        }
    }
    true
}

fn validate_script_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("path must not be empty");
    }
    if path.contains('\0') {
        return Err("path must not contain null bytes");
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("path must be relative, not absolute");
    }
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        return Err("path must not contain a drive letter");
    }
    if path.contains("..") {
        return Err("path must not contain directory traversal ('..')");
    }
    if !path.ends_with(".js") {
        return Err("script must have a .js file extension");
    }
    for component in path.split(&['/', '\\'][..]) {
        if component.is_empty() {
            return Err("path contains empty component");
        }
    }
    Ok(())
}

fn validate_preload_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("preload path must not be empty");
    }
    if path.contains('\0') {
        return Err("preload path must not contain null bytes");
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("preload path must be relative, not absolute");
    }
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        return Err("preload path must not contain a drive letter");
    }

    if let Some(framework_rel) = path.strip_prefix("../framework/") {
        if framework_rel.contains("..") {
            return Err("framework preload must not contain directory traversal ('..')");
        }
        if !framework_rel.ends_with(".js") {
            return Err("framework preload script must have .js extension");
        }
        return Ok(());
    }

    if path.starts_with("../") {
        return Err("cross-directory preload is only permitted to '../framework/'");
    }

    if path.contains("..") {
        return Err("preload path must not contain directory traversal ('..')");
    }

    if !path.ends_with(".js") {
        return Err("preload script must have .js extension");
    }

    Ok(())
}

fn is_valid_permission_name(perm: &str) -> bool {
    if perm.is_empty() || perm.len() > 128 {
        return false;
    }
    if !perm.contains('.') {
        return false;
    }
    if perm.starts_with('.') || perm.ends_with('.') {
        return false;
    }
    let segments: Vec<&str> = perm.split('.').collect();
    if segments.len() < 2 {
        return false;
    }
    for seg in segments {
        if seg.is_empty() {
            return false;
        }
        if !seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    true
}
