use crate::error::PackageError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The architectural kind of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManifestPackageKind {
    /// Background system service or data provider.
    Service,
    /// Native GPU visual widget rendered into the scene graph.
    Widget,
}

impl std::fmt::Display for ManifestPackageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service => write!(f, "service"),
            Self::Widget => write!(f, "widget"),
        }
    }
}

/// Metadata section `[package]` in `PackageManifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPackageInfo {
    /// Unique reverse-DNS identifier (e.g. `"com.sniffer.perf"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semantic version string (e.g. `"0.1.0"`).
    pub version: String,
    /// Package kind: `"service"` or `"widget"`.
    pub kind: ManifestPackageKind,
    /// Human-readable description of what the package provides.
    #[serde(default)]
    pub description: String,
    /// Authors and maintainers.
    #[serde(default)]
    pub authors: Vec<String>,
    /// SPDX license identifier (e.g. `"Apache-2.0"`).
    #[serde(default)]
    pub license: Option<String>,
    /// Minimum compatible SnifferLauncher engine version.
    #[serde(default = "default_min_engine")]
    pub min_engine_version: String,
}

fn default_min_engine() -> String {
    "0.1.0".to_string()
}

/// Dynamic entry point configuration `[entry]` in `PackageManifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// C-ABI export symbol to invoke when loading dynamically.
    /// Defaults to `"_sniffer_pkg_create"`.
    #[serde(default = "default_symbol")]
    pub symbol: String,
    /// Name of the compiled shared library (without file extension).
    /// If omitted, defaults to the package ID or folder name.
    #[serde(default)]
    pub library_name: Option<String>,
}

impl Default for ManifestEntry {
    fn default() -> Self {
        Self {
            symbol: default_symbol(),
            library_name: None,
        }
    }
}

fn default_symbol() -> String {
    "_sniffer_pkg_create".to_string()
}

/// Security permissions requested by the package `[permissions]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestPermissions {
    /// List of system capabilities (e.g. `["system.metrics", "system.storage"]`).
    #[serde(default)]
    pub system: Vec<String>,
}

/// Visual widget properties `[widget]` in `PackageManifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWidgetConfig {
    /// Default Z-index in the render pass (higher is drawn on top).
    #[serde(default)]
    pub default_z_index: i32,
    /// Whether this widget respects parent clipping boundaries.
    #[serde(default = "default_true")]
    pub supports_clipping: bool,
    /// Whether this widget respects parent coordinate transformations.
    #[serde(default = "default_true")]
    pub supports_transform: bool,
}

impl Default for ManifestWidgetConfig {
    fn default() -> Self {
        Self {
            default_z_index: 0,
            supports_clipping: true,
            supports_transform: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Full deserialized representation of a `PackageManifest.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// `[package]` metadata section.
    pub package: ManifestPackageInfo,
    /// `[entry]` dynamic symbol and library configuration.
    #[serde(default)]
    pub entry: ManifestEntry,
    /// `[permissions]` requested by the package.
    #[serde(default)]
    pub permissions: ManifestPermissions,
    /// `[widget]` configuration (present only if `kind == "widget"`).
    #[serde(default)]
    pub widget: Option<ManifestWidgetConfig>,
    /// `[dependencies]` required other package IDs mapped to version requirements.
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// `[metadata]` custom JSON/TOML configuration passed to the package.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Absolute directory where `PackageManifest.toml` was loaded from.
    #[serde(skip)]
    pub root_dir: PathBuf,
}

impl PackageManifest {
    /// Parses a `PackageManifest.toml` file from the specified path.
    ///
    /// # Errors
    /// Returns [`PackageError::Io`] on read failure or
    /// [`PackageError::InvalidManifest`] on parse/validation failure.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let mut manifest: Self = toml::from_str(&content)
            .map_err(|e| PackageError::InvalidManifest(format!("TOML syntax error: {e}")))?;

        manifest.root_dir = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parses a `PackageManifest.toml` from an in-memory TOML string.
    ///
    /// # Errors
    /// Returns [`PackageError::InvalidManifest`] on syntax or validation failure.
    pub fn parse_str(content: &str) -> Result<Self, PackageError> {
        let manifest: Self = toml::from_str(content)
            .map_err(|e| PackageError::InvalidManifest(format!("TOML syntax error: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates all required fields, SemVer strings, and ID conventions.
    ///
    /// # Errors
    /// Returns [`PackageError::InvalidManifest`] describing any validation error.
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.package.id.trim().is_empty() {
            return Err(PackageError::InvalidManifest(
                "package.id cannot be empty".to_string(),
            ));
        }
        if self.package.name.trim().is_empty() {
            return Err(PackageError::InvalidManifest(
                "package.name cannot be empty".to_string(),
            ));
        }
        if semver::Version::parse(&self.package.version).is_err() {
            return Err(PackageError::InvalidManifest(format!(
                "invalid semantic version in package.version: '{}'",
                self.package.version
            )));
        }
        if semver::VersionReq::parse(&self.package.min_engine_version).is_err()
            && semver::Version::parse(&self.package.min_engine_version).is_err()
        {
            return Err(PackageError::InvalidManifest(format!(
                "invalid min_engine_version requirement: '{}'",
                self.package.min_engine_version
            )));
        }

        // Validate dependency version requirements
        for (dep_id, ver_req) in &self.dependencies {
            if dep_id.trim().is_empty() {
                return Err(PackageError::InvalidManifest(
                    "dependency ID cannot be empty".to_string(),
                ));
            }
            if semver::VersionReq::parse(ver_req).is_err() {
                return Err(PackageError::InvalidManifest(format!(
                    "invalid SemVer requirement for dependency '{dep_id}': '{ver_req}'"
                )));
            }
        }

        Ok(())
    }
}
