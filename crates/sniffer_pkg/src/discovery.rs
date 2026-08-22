//! Package discovery and topological dependency resolution engine.
//!
//! Scans a packages root directory (e.g. `packages/` or `/data/data/.../packages/`),
//! discovers `PackageManifest.toml` manifests, validates SemVer dependency graphs,
//! and orders packages topologically for safe initialization.

use crate::error::PackageError;
#[cfg(feature = "dynamic")]
use crate::manifest::ManifestPackageKind;
use crate::manifest::PackageManifest;
use crate::registry::PackageRegistry;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// Directory scanner and dependency graph resolver for SnifferLauncher packages.
pub struct PackageDiscovery;

impl PackageDiscovery {
    /// Scans the given directory for subdirectories containing a `PackageManifest.toml`.
    ///
    /// Silently ignores directories that do not contain a manifest or whose
    /// manifest fails syntax validation (log warnings can be hooked).
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

    /// Resolves the dependency graph of the given manifests and returns them
    /// sorted in topological order (dependencies appear before dependents).
    ///
    /// # Errors
    /// Returns [`PackageError::DependencyResolutionFailed`] if:
    /// - A required dependency is missing from the manifest set.
    /// - A dependency's version does not satisfy the declared `VersionReq`.
    /// - A cyclic dependency loop is detected.
    pub fn resolve_dependencies(
        manifests: &[PackageManifest],
    ) -> Result<Vec<PackageManifest>, PackageError> {
        let mut by_id: HashMap<&str, &PackageManifest> = HashMap::new();
        for m in manifests {
            if by_id.insert(&m.package.id, m).is_some() {
                return Err(PackageError::DependencyResolutionFailed(format!(
                    "duplicate package ID: '{}'",
                    m.package.id
                )));
            }
        }

        // Validate dependencies exist and satisfy SemVer
        for m in manifests {
            for (dep_id, req_str) in &m.dependencies {
                let Some(dep_manifest) = by_id.get(dep_id.as_str()) else {
                    return Err(PackageError::DependencyResolutionFailed(format!(
                        "package '{}' requires missing dependency '{}'",
                        m.package.id, dep_id
                    )));
                };

                let req = semver::VersionReq::parse(req_str).map_err(|e| {
                    PackageError::DependencyResolutionFailed(format!(
                        "invalid SemVer requirement '{req_str}' for dep '{dep_id}': {e}"
                    ))
                })?;

                let dep_version =
                    semver::Version::parse(&dep_manifest.package.version).map_err(|e| {
                        PackageError::DependencyResolutionFailed(format!(
                            "invalid SemVer version on package '{dep_id}': {e}"
                        ))
                    })?;

                if !req.matches(&dep_version) {
                    return Err(PackageError::DependencyResolutionFailed(format!(
                        "package '{}' requires '{}' {}, but found version {}",
                        m.package.id, dep_id, req_str, dep_manifest.package.version
                    )));
                }
            }
        }

        // Kahn's Algorithm for Topological Sort
        // In-degree: number of dependencies that node depends on
        // Adjacency: dep_id -> list of packages that depend on dep_id
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents_of: HashMap<&str, Vec<&str>> = HashMap::new();

        for m in manifests {
            let id = m.package.id.as_str();
            in_degree.insert(id, m.dependencies.len());
            dependents_of.entry(id).or_default();

            for dep_id in m.dependencies.keys() {
                dependents_of.entry(dep_id.as_str()).or_default().push(id);
            }
        }

        // Queue all nodes with 0 incoming dependencies
        let mut queue: VecDeque<&str> = VecDeque::new();
        for (&id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(id);
            }
        }

        let mut sorted_ids = Vec::with_capacity(manifests.len());

        while let Some(current) = queue.pop_front() {
            sorted_ids.push(current);

            if let Some(deps) = dependents_of.get(current) {
                for &dependent in deps {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dependent);
                        }
                    }
                }
            }
        }

        if sorted_ids.len() < manifests.len() {
            let remaining: Vec<&str> = in_degree
                .iter()
                .filter(|&(_, deg)| *deg > 0)
                .map(|(&id, _)| id)
                .collect();
            return Err(PackageError::DependencyResolutionFailed(format!(
                "dependency cycle detected among packages: {remaining:?}"
            )));
        }

        let sorted_manifests = sorted_ids
            .into_iter()
            .map(|id| (*by_id.get(id).unwrap()).clone())
            .collect();

        Ok(sorted_manifests)
    }

    /// Dynamically loads a package described by `manifest` into `registry`.
    ///
    /// # Errors
    /// Returns [`PackageError`] if dynamic loading is disabled or fails to load.
    pub fn load_manifest_into_registry(
        manifest: &PackageManifest,
        registry: &mut PackageRegistry,
    ) -> Result<(), PackageError> {
        #[cfg(feature = "dynamic")]
        {
            let lib_name = manifest
                .entry
                .library_name
                .as_deref()
                .unwrap_or(&manifest.package.id);

            let dll_ext = std::env::consts::DLL_EXTENSION;
            let lib_file_name = if cfg!(target_os = "windows") {
                format!("{lib_name}.{dll_ext}")
            } else {
                format!("lib{lib_name}.{dll_ext}")
            };

            let candidates = [
                manifest.root_dir.join(&lib_file_name),
                manifest
                    .root_dir
                    .join("target")
                    .join("debug")
                    .join(&lib_file_name),
                manifest
                    .root_dir
                    .join("target")
                    .join("release")
                    .join(&lib_file_name),
                manifest
                    .root_dir
                    .join("..")
                    .join("target")
                    .join("debug")
                    .join(&lib_file_name),
                manifest
                    .root_dir
                    .join("..")
                    .join("target")
                    .join("release")
                    .join(&lib_file_name),
            ];

            let lib_path = candidates
                .into_iter()
                .find(|p| p.is_file())
                .unwrap_or_else(|| manifest.root_dir.join(&lib_file_name));

            match manifest.package.kind {
                ManifestPackageKind::Service => {
                    let (pkg, handle) = unsafe {
                        crate::loader::load_service_dylib_with_symbol(
                            &lib_path,
                            &manifest.entry.symbol,
                        )?
                    };
                    registry.register_dynamic_service(pkg, handle);
                }
                ManifestPackageKind::Widget => {
                    let (pkg, handle) = unsafe {
                        crate::loader::load_widget_dylib_with_symbol(
                            &lib_path,
                            &manifest.entry.symbol,
                        )?
                    };
                    registry.register_dynamic_widget(pkg, handle);
                }
            }

            Ok(())
        }

        #[cfg(not(feature = "dynamic"))]
        {
            let _ = (manifest, registry);
            Err(PackageError::Init(
                "dynamic loading is disabled in this build (enable 'dynamic' feature)".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_manifest(id: &str, version: &str, deps: &[(&str, &str)]) -> PackageManifest {
        let toml_str = format!(
            r#"
[package]
id = "{id}"
name = "Mock {id}"
version = "{version}"
kind = "service"

[dependencies]
{}
"#,
            deps.iter()
                .map(|(d, v)| format!("\"{d}\" = \"{v}\""))
                .collect::<Vec<_>>()
                .join("\n")
        );

        PackageManifest::parse_str(&toml_str).expect("mock manifest must be valid")
    }

    #[test]
    fn topological_sort_linear_chain() {
        // C -> B -> A (A depends on B, B depends on C)
        let pkg_a = create_mock_manifest("pkg.a", "1.0.0", &[("pkg.b", ">=1.0.0")]);
        let pkg_b = create_mock_manifest("pkg.b", "1.0.0", &[("pkg.c", ">=1.0.0")]);
        let pkg_c = create_mock_manifest("pkg.c", "1.0.0", &[]);

        let input = vec![pkg_a, pkg_c, pkg_b];
        let sorted = PackageDiscovery::resolve_dependencies(&input).expect("must resolve");

        let order: Vec<&str> = sorted.iter().map(|m| m.package.id.as_str()).collect();
        assert_eq!(order, vec!["pkg.c", "pkg.b", "pkg.a"]);
    }

    #[test]
    fn topological_sort_diamond_dependency() {
        // Core -> Auth, Core -> Db -> Auth -> App
        let core = create_mock_manifest("pkg.core", "1.0.0", &[]);
        let db = create_mock_manifest("pkg.db", "1.0.0", &[("pkg.core", "^1.0")]);
        let auth = create_mock_manifest("pkg.auth", "1.0.0", &[("pkg.core", "^1.0")]);
        let app = create_mock_manifest(
            "pkg.app",
            "1.0.0",
            &[("pkg.db", "^1.0"), ("pkg.auth", "^1.0")],
        );

        let input = vec![app, auth, core, db];
        let sorted = PackageDiscovery::resolve_dependencies(&input).expect("must resolve");

        let ids: Vec<&str> = sorted.iter().map(|m| m.package.id.as_str()).collect();
        assert_eq!(ids[0], "pkg.core");
        assert_eq!(ids[3], "pkg.app");
    }

    #[test]
    fn missing_dependency_fails() {
        let pkg_a = create_mock_manifest("pkg.a", "1.0.0", &[("pkg.missing", ">=1.0.0")]);
        let err = PackageDiscovery::resolve_dependencies(&[pkg_a]).unwrap_err();
        assert!(matches!(err, PackageError::DependencyResolutionFailed(_)));
    }

    #[test]
    fn incompatible_version_fails() {
        let pkg_a = create_mock_manifest("pkg.a", "1.0.0", &[("pkg.b", ">=2.0.0")]);
        let pkg_b = create_mock_manifest("pkg.b", "1.5.0", &[]);

        let err = PackageDiscovery::resolve_dependencies(&[pkg_a, pkg_b]).unwrap_err();
        assert!(matches!(err, PackageError::DependencyResolutionFailed(_)));
    }

    #[test]
    fn cyclic_dependency_fails() {
        // A -> B -> A
        let pkg_a = create_mock_manifest("pkg.a", "1.0.0", &[("pkg.b", ">=1.0.0")]);
        let pkg_b = create_mock_manifest("pkg.b", "1.0.0", &[("pkg.a", ">=1.0.0")]);

        let err = PackageDiscovery::resolve_dependencies(&[pkg_a, pkg_b]).unwrap_err();
        assert!(matches!(err, PackageError::DependencyResolutionFailed(_)));
    }

    #[test]
    fn scan_dir_discovers_manifests() {
        let tmp = std::env::temp_dir().join(format!(
            "sniffer_scan_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pkg1_dir = tmp.join("pkg1");
        let pkg2_dir = tmp.join("pkg2");
        std::fs::create_dir_all(&pkg1_dir).unwrap();
        std::fs::create_dir_all(&pkg2_dir).unwrap();

        std::fs::write(
            pkg1_dir.join("PackageManifest.toml"),
            r#"
[package]
id = "com.mock.one"
name = "Mock One"
version = "0.1.0"
kind = "service"
"#,
        )
        .unwrap();

        std::fs::write(
            pkg2_dir.join("PackageManifest.toml"),
            r#"
[package]
id = "com.mock.two"
name = "Mock Two"
version = "0.2.0"
kind = "widget"
"#,
        )
        .unwrap();

        let manifests = PackageDiscovery::scan_dir(&tmp);
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].package.id, "com.mock.one");
        assert_eq!(manifests[1].package.id, "com.mock.two");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
