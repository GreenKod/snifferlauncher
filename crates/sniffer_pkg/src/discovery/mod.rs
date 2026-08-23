//! Package discovery and topological dependency resolution engine.
//!
//! Scans a packages root directory (e.g. `packages/` or `/data/data/.../packages/`),
//! discovers `PackageManifest.toml` manifests, validates SemVer dependency graphs,
//! and orders packages topologically for safe initialization.

pub mod loader;
pub mod resolver;
pub mod scanner;

#[cfg(test)]
mod tests;

use crate::error::PackageError;
use crate::manifest::PackageManifest;
use crate::registry::PackageRegistry;
use std::path::Path;

/// Directory scanner and dependency graph resolver for SnifferLauncher packages.
pub struct PackageDiscovery;

impl PackageDiscovery {
    /// Scans the given directory for subdirectories containing a `PackageManifest.toml`.
    #[must_use]
    pub fn scan_dir(dir: impl AsRef<Path>) -> Vec<PackageManifest> {
        scanner::scan_dir(dir)
    }

    /// Resolves the dependency graph of the given manifests and returns them
    /// sorted in topological order (dependencies appear before dependents).
    pub fn resolve_dependencies(
        manifests: &[PackageManifest],
    ) -> Result<Vec<PackageManifest>, PackageError> {
        resolver::resolve_dependencies(manifests)
    }

    /// Dynamically loads a package described by `manifest` into `registry`.
    pub fn load_manifest_into_registry(
        manifest: &PackageManifest,
        registry: &mut PackageRegistry,
    ) -> Result<(), PackageError> {
        loader::load_manifest_into_registry(manifest, registry)
    }
}
