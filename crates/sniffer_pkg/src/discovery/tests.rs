use super::*;
use crate::error::PackageError;

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

#[cfg(feature = "dynamic")]
#[test]
fn test_load_manifest_sha256_mismatch_fails() {
    let tmp = std::env::temp_dir().join(format!(
        "sniffer_sha_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let dll_ext = std::env::consts::DLL_EXTENSION;
    let lib_file_name = if cfg!(target_os = "windows") {
        format!("com.mock.corrupt.{dll_ext}")
    } else {
        format!("libcom.mock.corrupt.{dll_ext}")
    };

    let fake_lib = tmp.join(&lib_file_name);
    std::fs::write(&fake_lib, b"corrupted payload bytes").unwrap();

    let manifest_content = r#"
[package]
id = "com.mock.corrupt"
name = "Mock Corrupt"
version = "0.1.0"
kind = "service"
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
"#;
    let mut manifest = PackageManifest::parse_str(manifest_content).unwrap();
    manifest.root_dir = tmp.clone();

    let vault = std::sync::Arc::new(sniffer_core::DataVault::new(None));
    let mut registry = PackageRegistry::new(vault);
    let res = PackageDiscovery::load_manifest_into_registry(&manifest, &mut registry);

    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("SHA-256 integrity verification failed"));

    let _ = std::fs::remove_dir_all(&tmp);
}
