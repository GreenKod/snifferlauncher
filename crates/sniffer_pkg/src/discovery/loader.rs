use crate::error::PackageError;
use crate::manifest::PackageManifest;
use crate::registry::PackageRegistry;

/// Dynamically loads a package described by `manifest` into `registry`.
///
/// Implements full ADR 001 verification:
/// 1. Path resolution & boundary containment
/// 2. Cryptographic SHA-256 integrity verification (if declared in manifest)
/// 3. C ABI v1 loading with header handshake ("SNIF", v1) and fallback
///
/// # Errors
/// Returns [`PackageError`] if dynamic loading is disabled, binary fails integrity,
/// or fails to load.
pub fn load_manifest_into_registry(
    manifest: &PackageManifest,
    registry: &mut PackageRegistry,
) -> Result<(), PackageError> {
    #[cfg(feature = "dynamic")]
    {
        use crate::manifest::ManifestPackageKind;

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

        let Some(lib_path) = candidates.into_iter().find(|p| p.is_file()) else {
            return Err(PackageError::DynLoad(format!(
                "Native library '{}' not found in search paths for package '{}'",
                lib_file_name, manifest.package.id
            )));
        };

        // 1. Path Containment Check: Verify candidate does not escape repository root
        if let (Ok(canonical_lib), Ok(canonical_root)) =
            (lib_path.canonicalize(), manifest.root_dir.canonicalize())
        {
            let root_parent = canonical_root.parent().unwrap_or(&canonical_root);
            if !canonical_lib.starts_with(&canonical_root)
                && !canonical_lib.starts_with(root_parent)
            {
                return Err(PackageError::DynLoad(format!(
                    "Security: Library path '{}' escapes authorized package root boundaries",
                    lib_path.display()
                )));
            }
        }

        // 2. Cryptographic SHA-256 Digest Integrity Verification
        if let Some(ref expected_sha) = manifest.package.sha256 {
            let bytes = std::fs::read(&lib_path).map_err(|e| {
                PackageError::DynLoad(format!(
                    "Failed to read library '{}' for integrity check: {e}",
                    lib_path.display()
                ))
            })?;
            let actual_sha = crate::manifest::compute_sha256_hex(&bytes);
            if !actual_sha.eq_ignore_ascii_case(expected_sha.trim()) {
                return Err(PackageError::DynLoad(format!(
                    "Security: SHA-256 integrity verification failed for package '{}': expected '{}', computed '{}'",
                    manifest.package.id,
                    expected_sha.trim(),
                    actual_sha
                )));
            }
        }

        // 3. Dynamic library loading & C ABI handshake
        match manifest.package.kind {
            ManifestPackageKind::Service => {
                // Prioritize C ABI v1 loader
                let c_abi_res = unsafe {
                    crate::loader::load_c_abi_package(&lib_path, Some(&manifest.entry.symbol))
                };
                match c_abi_res {
                    Ok((pkg, handle)) => {
                        registry.register_dynamic_service(pkg, handle);
                    }
                    Err(_) => {
                        // Fallback to legacy service loader
                        let (pkg, handle) = unsafe {
                            crate::loader::load_service_dylib_with_symbol(
                                &lib_path,
                                &manifest.entry.symbol,
                            )?
                        };
                        registry.register_dynamic_service(pkg, handle);
                    }
                }
            }
            ManifestPackageKind::Widget => {
                let (pkg, handle) = unsafe {
                    crate::loader::load_widget_dylib_with_symbol(&lib_path, &manifest.entry.symbol)?
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
