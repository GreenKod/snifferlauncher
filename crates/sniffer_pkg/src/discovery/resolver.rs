use crate::error::PackageError;
use crate::manifest::PackageManifest;
use std::collections::{HashMap, VecDeque};

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
