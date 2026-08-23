use crate::manifest::PackageManifest;
use std::path::Path;

/// Scans the given directory for subdirectories containing a `PackageManifest.toml`.
///
/// Silently ignores directories that do not contain a manifest or whose
/// manifest fails syntax validation.
#[must_use]
pub fn scan_dir(dir: impl AsRef<Path>) -> Vec<PackageManifest> {
    let dir = dir.as_ref();
    let mut manifests = Vec::new();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return manifests;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("PackageManifest.toml");
            if manifest_path.is_file() {
                if let Ok(manifest) = PackageManifest::load_from_file(&manifest_path) {
                    manifests.push(manifest);
                }
            }
        }
    }

    // Deterministic sorting by package ID for consistent test and load order
    manifests.sort_by(|a, b| a.package.id.cmp(&b.package.id));
    manifests
}
