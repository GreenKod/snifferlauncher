//! Dynamic library loading for the package layer.
//!
//! This module abstracts two loading strategies:
//!
//! | Strategy | API                                     |
//! |----------|-----------------------------------------|
//! | Static   | Direct `Arc<dyn …>` registration        |
//! | Dynamic  | [`DynamicHandle`] via `libloading`      |
//!
//! # Dynamic Package ABI Contract
//!
//! A dynamic package is a `cdylib` crate that **must** export the following
//! symbols with `#[no_mangle] pub unsafe extern "C"`:
//!
//! ## Service packages (`LauncherPackage`)
//!
//! ```rust,ignore
//! // In your cdylib crate — compiled with the same toolchain as the host.
//! // Rust 2024: use #[unsafe(no_mangle)] instead of #[no_mangle].
//! #[unsafe(no_mangle)]
//! pub unsafe extern "C" fn pkg_create() -> *mut Box<dyn LauncherPackage> {
//!     Box::into_raw(Box::new(Box::new(MyPkg::new()) as Box<dyn LauncherPackage>))
//! }
//!
//! #[unsafe(no_mangle)]
//! pub unsafe extern "C" fn pkg_destroy(ptr: *mut Box<dyn LauncherPackage>) {
//!     if !ptr.is_null() {
//!         unsafe { drop(Box::from_raw(ptr)); }
//!     }
//! }
//! ```
//!
//! ## Widget packages (`WidgetPackage`)
//!
//! Same pattern but symbol names are `pkg_create_widget` / `pkg_destroy_widget`.
//!
//! # ABI Stability Warning
//!
//! Both the host and the dynamic library **must** be compiled with the same
//! Rust toolchain version and the same version of `sniffer_pkg`.  Mixing
//! versions is **undefined behaviour**.
//!
//! # Android Notes
//!
//! On Android, `dlopen` only works on files inside the app's internal
//! directories or pre-installed in `/system/lib`.  Use
//! [`prepare_android_dylib`] to copy a `.so` file into the app's internal
//! packages directory and apply the required `chmod 0555` permissions before
//! calling `load_dynamic`.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// DynamicHandle
// ---------------------------------------------------------------------------

/// RAII guard that keeps a dynamically loaded library alive.
///
/// When this value is dropped the library is unloaded via `dlclose` /
/// `FreeLibrary`.  Always drop [`DynamicHandle`] **after** all
/// `Arc<dyn LauncherPackage>` or `Arc<dyn WidgetPackage>` clones that were
/// created from it have been dropped.
///
/// [`PackageRegistry`][crate::registry::PackageRegistry] guarantees this
/// ordering by storing handles in a `Vec` that is cleared last in
/// [`unload_all`][crate::registry::PackageRegistry::unload_all].
#[cfg(feature = "dynamic")]
pub struct DynamicHandle {
    // The library must be kept alive as long as any symbol from it is in use.
    _lib: libloading::Library,
}

#[cfg(feature = "dynamic")]
impl std::fmt::Debug for DynamicHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicHandle").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// C-ABI function pointer types
// ---------------------------------------------------------------------------

#[cfg(feature = "dynamic")]
type ServiceCreateFn = unsafe extern "C" fn() -> *mut Box<dyn crate::package::LauncherPackage>;

#[cfg(feature = "dynamic")]
#[allow(dead_code)]
type ServiceDestroyFn = unsafe extern "C" fn(*mut Box<dyn crate::package::LauncherPackage>);

#[cfg(feature = "dynamic")]
type WidgetCreateFn = unsafe extern "C" fn() -> *mut Box<dyn crate::package::WidgetPackage>;

#[cfg(feature = "dynamic")]
#[allow(dead_code)]
type WidgetDestroyFn = unsafe extern "C" fn(*mut Box<dyn crate::package::WidgetPackage>);

// ---------------------------------------------------------------------------
// Dynamic loading helpers
// ---------------------------------------------------------------------------

/// Load a service ([`LauncherPackage`][crate::package::LauncherPackage]) from
/// a shared library at `path`.
///
/// # Safety
///
/// The shared library at `path` **must** export `pkg_create` and
/// `pkg_destroy` with the exact C-ABI signatures documented in this module.
/// The library and the host must be compiled with the same toolchain and
/// `sniffer_pkg` version.
///
/// # Errors
///
/// Returns [`PackageError::DynLoad`] if the library cannot be opened or the
/// required symbols are not found.
#[cfg(feature = "dynamic")]
pub unsafe fn load_service_dylib(
    path: &Path,
) -> Result<
    (
        std::sync::Arc<dyn crate::package::LauncherPackage>,
        DynamicHandle,
    ),
    crate::error::PackageError,
> {
    unsafe { load_service_dylib_with_symbol(path, "pkg_create") }
}

/// Load a service from a shared library with a custom entry symbol name.
///
/// # Safety
/// Caller guarantees ABI compatibility.
#[cfg(feature = "dynamic")]
pub unsafe fn load_service_dylib_with_symbol(
    path: &Path,
    symbol_name: &str,
) -> Result<
    (
        std::sync::Arc<dyn crate::package::LauncherPackage>,
        DynamicHandle,
    ),
    crate::error::PackageError,
> {
    // SAFETY: caller guarantees ABI contract.
    let lib = unsafe { libloading::Library::new(path) }?;

    let sym_bytes = std::ffi::CString::new(symbol_name).map_err(|e| {
        crate::error::PackageError::DynLoad(format!("invalid symbol name '{symbol_name}': {e}"))
    })?;

    let raw: *mut Box<dyn crate::package::LauncherPackage> = {
        let sym_res: Result<libloading::Symbol<ServiceCreateFn>, _> =
            unsafe { lib.get(sym_bytes.as_bytes_with_nul()) };

        match sym_res {
            Ok(create) => unsafe { create() },
            Err(_) => {
                // Fallback to pkg_create if custom symbol wasn't found
                let fallback: libloading::Symbol<ServiceCreateFn> =
                    unsafe { lib.get(b"pkg_create\0") }?;
                unsafe { fallback() }
            }
        }
    };

    if raw.is_null() {
        return Err(crate::error::PackageError::DynLoad(format!(
            "symbol '{symbol_name}' returned null"
        )));
    }

    let pkg: Box<dyn crate::package::LauncherPackage> = unsafe { *Box::from_raw(raw) };
    let arc = std::sync::Arc::from(pkg);

    Ok((arc, DynamicHandle { _lib: lib }))
}

/// Load a widget ([`WidgetPackage`][crate::package::WidgetPackage]) from a
/// shared library at `path`.
///
/// # Safety
///
/// Same requirements as [`load_service_dylib`].
///
/// # Errors
///
/// Returns [`PackageError::DynLoad`] if the library cannot be opened or the
/// required symbols are not found.
#[cfg(feature = "dynamic")]
pub unsafe fn load_widget_dylib(
    path: &Path,
) -> Result<
    (
        std::sync::Arc<dyn crate::package::WidgetPackage>,
        DynamicHandle,
    ),
    crate::error::PackageError,
> {
    unsafe { load_widget_dylib_with_symbol(path, "pkg_create_widget") }
}

/// Load a widget from a shared library with a custom entry symbol name.
///
/// # Safety
/// Caller guarantees ABI compatibility.
#[cfg(feature = "dynamic")]
pub unsafe fn load_widget_dylib_with_symbol(
    path: &Path,
    symbol_name: &str,
) -> Result<
    (
        std::sync::Arc<dyn crate::package::WidgetPackage>,
        DynamicHandle,
    ),
    crate::error::PackageError,
> {
    // SAFETY: caller guarantees ABI contract.
    let lib = unsafe { libloading::Library::new(path) }?;

    let sym_bytes = std::ffi::CString::new(symbol_name).map_err(|e| {
        crate::error::PackageError::DynLoad(format!("invalid symbol name '{symbol_name}': {e}"))
    })?;

    let raw: *mut Box<dyn crate::package::WidgetPackage> = {
        let sym_res: Result<libloading::Symbol<WidgetCreateFn>, _> =
            unsafe { lib.get(sym_bytes.as_bytes_with_nul()) };

        match sym_res {
            Ok(create) => unsafe { create() },
            Err(_) => {
                // Fallback to pkg_create_widget if custom symbol wasn't found
                let fallback: libloading::Symbol<WidgetCreateFn> =
                    unsafe { lib.get(b"pkg_create_widget\0") }?;
                unsafe { fallback() }
            }
        }
    };

    if raw.is_null() {
        return Err(crate::error::PackageError::DynLoad(format!(
            "symbol '{symbol_name}' returned null"
        )));
    }

    // SAFETY: `raw` was just returned from widget create function.
    let pkg: Box<dyn crate::package::WidgetPackage> = unsafe { *Box::from_raw(raw) };
    let arc = std::sync::Arc::from(pkg);

    Ok((arc, DynamicHandle { _lib: lib }))
}

// ---------------------------------------------------------------------------
// Android internal-directory helper
// ---------------------------------------------------------------------------

/// Copies a `.so` file into the app's internal packages directory and sets
/// the permissions required for `dlopen` on Android (`chmod 0555`).
///
/// **Call this function before [`crate::registry::PackageRegistry::load_dynamic`].**
///
/// The destination directory is:
/// ```text
/// <internal_files_dir>/packages/<library_filename>
/// ```
///
/// where `internal_files_dir` is typically
/// `/data/data/com.sniffer.launcher/files` on a production device.
///
/// # Errors
///
/// Returns [`PackageError::Io`] on any file-system error.
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
    // This is the minimum permission set accepted by the Android dynamic linker
    // for files outside of the APK's native library directory.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o555))?;
    }

    Ok(dest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_android_dylib_creates_dir_and_file() {
        let tmp = std::env::temp_dir().join("sniffer_pkg_loader_test");
        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let src_file = src_dir.join("libfoo.so");
        std::fs::write(&src_file, b"ELF").unwrap();

        let internal = tmp.join("internal");
        let dest = prepare_android_dylib(&src_file, &internal).unwrap();

        assert!(dest.exists(), "destination file must exist");
        assert_eq!(dest.file_name().unwrap(), "libfoo.so");

        // Clean up.
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
