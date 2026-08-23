//! Android dynamic library filesystem helper.

use std::path::{Path, PathBuf};

/// Copies a `.so` file into the app's internal packages directory and sets
/// the permissions required for `dlopen` on Android (`chmod 0555`).
///
/// **Call this function before [`crate::registry::PackageRegistry::load_dynamic`].**
///
/// # Errors
///
/// Returns [`PackageError::Io`][crate::error::PackageError::Io] on any file-system error.
pub fn prepare_android_dylib(
    source_path: &Path,
    internal_files_dir: &Path,
) -> Result<PathBuf, crate::error::PackageError> {
    let file_name = source_path.file_name().ok_or_else(|| {
        crate::error::PackageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "source path has no file name",
        ))
    })?;

    let packages_dir = internal_files_dir.join("packages");
    std::fs::create_dir_all(&packages_dir)?;

    let dest = packages_dir.join(file_name);
    std::fs::copy(source_path, &dest)?;

    // Apply 0555 (r-xr-xr-x) so that dlopen can map the file into memory.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o555))?;
    }

    Ok(dest)
}
