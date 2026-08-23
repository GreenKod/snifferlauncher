use crate::error::PackageError;
use crate::manifest::PackageManifest;
use crate::registry::PackageRegistry;

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
