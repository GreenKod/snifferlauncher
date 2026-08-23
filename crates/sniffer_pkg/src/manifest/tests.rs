use super::schema::*;
use crate::error::PackageError;

#[test]
fn parse_valid_service_manifest() {
    let toml_str = r#"
[package]
id = "com.sniffer.perf"
name = "System Performance Monitor"
version = "0.1.0"
kind = "service"
description = "Real-time metrics"
authors = ["Sniffer Core Team"]
license = "Apache-2.0"
min_engine_version = "0.1.0"

[entry]
symbol = "_sniffer_pkg_create"
library_name = "pkg_perfmon"

[permissions]
system = ["system.metrics", "system.storage"]

[dependencies]
"com.sniffer.vault" = ">=0.1.0"
"#;

    let manifest = PackageManifest::parse_str(toml_str).expect("must parse successfully");
    assert_eq!(manifest.package.id, "com.sniffer.perf");
    assert_eq!(manifest.package.name, "System Performance Monitor");
    assert_eq!(manifest.package.version, "0.1.0");
    assert_eq!(manifest.package.kind, ManifestPackageKind::Service);
    assert_eq!(manifest.entry.symbol, "_sniffer_pkg_create");
    assert_eq!(manifest.entry.library_name.as_deref(), Some("pkg_perfmon"));
    assert_eq!(
        manifest.permissions.system,
        vec!["system.metrics", "system.storage"]
    );
    assert_eq!(
        manifest.dependencies.get("com.sniffer.vault").unwrap(),
        ">=0.1.0"
    );
}

#[test]
fn parse_valid_widget_manifest() {
    let toml_str = r#"
[package]
id = "com.sniffer.scroll_view"
name = "Native GPU ScrollView"
version = "1.2.3"
kind = "widget"

[widget]
default_z_index = 5
supports_clipping = true
supports_transform = false
"#;

    let manifest = PackageManifest::parse_str(toml_str).expect("must parse successfully");
    assert_eq!(manifest.package.id, "com.sniffer.scroll_view");
    assert_eq!(manifest.package.kind, ManifestPackageKind::Widget);
    let widget_cfg = manifest.widget.expect("widget config must be present");
    assert_eq!(widget_cfg.default_z_index, 5);
    assert!(widget_cfg.supports_clipping);
    assert!(!widget_cfg.supports_transform);
}

#[test]
fn invalid_version_fails_validation() {
    let toml_str = r#"
[package]
id = "com.sniffer.test"
name = "Test"
version = "invalid-version"
kind = "service"
"#;

    let err = PackageManifest::parse_str(toml_str).unwrap_err();
    assert!(matches!(err, PackageError::InvalidManifest(_)));
}

#[test]
fn empty_id_fails_validation() {
    let toml_str = r#"
[package]
id = "   "
name = "Test"
version = "1.0.0"
kind = "service"
"#;

    let err = PackageManifest::parse_str(toml_str).unwrap_err();
    assert!(matches!(err, PackageError::InvalidManifest(_)));
}

#[test]
fn invalid_dependency_requirement_fails() {
    let toml_str = r#"
[package]
id = "com.sniffer.test"
name = "Test"
version = "1.0.0"
kind = "service"

[dependencies]
"com.sniffer.other" = "not-a-semver-req"
"#;

    let err = PackageManifest::parse_str(toml_str).unwrap_err();
    assert!(matches!(err, PackageError::InvalidManifest(_)));
}
