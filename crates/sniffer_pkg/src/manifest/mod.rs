//! Standard package manifest parser and validator (`PackageManifest.toml`).
//!
//! Every isolated package under `packages/<pkg_name>/` or dynamic plugin folder
//! declares a `PackageManifest.toml` defining its metadata, type, entry point,
//! permissions, and dependencies.

pub mod schema;

#[cfg(test)]
mod tests;

pub use schema::{
    ManifestEntry, ManifestPackageInfo, ManifestPackageKind, ManifestPermissions,
    ManifestWidgetConfig, PackageManifest,
};
