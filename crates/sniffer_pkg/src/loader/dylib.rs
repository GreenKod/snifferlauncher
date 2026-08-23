#[cfg(feature = "dynamic")]
use std::path::Path;

/// RAII guard that keeps a dynamically loaded library alive.
#[cfg(feature = "dynamic")]
pub struct DynamicHandle {
    _lib: libloading::Library,
}

#[cfg(feature = "dynamic")]
impl std::fmt::Debug for DynamicHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicHandle").finish_non_exhaustive()
    }
}

#[cfg(feature = "dynamic")]
type ServiceCreateFn = unsafe extern "C" fn() -> *mut Box<dyn crate::package::LauncherPackage>;

#[cfg(feature = "dynamic")]
type WidgetCreateFn = unsafe extern "C" fn() -> *mut Box<dyn crate::package::WidgetPackage>;

/// Load a service ([`LauncherPackage`][crate::package::LauncherPackage]) from a shared library.
///
/// # Safety
/// Caller guarantees ABI compatibility.
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

/// Load a widget ([`WidgetPackage`][crate::package::WidgetPackage]) from a shared library.
///
/// # Safety
/// Caller guarantees ABI compatibility.
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

    let pkg: Box<dyn crate::package::WidgetPackage> = unsafe { *Box::from_raw(raw) };
    let arc = std::sync::Arc::from(pkg);

    Ok((arc, DynamicHandle { _lib: lib }))
}
