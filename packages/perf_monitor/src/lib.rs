//! `pkg_perfmon` — System Performance Monitor Package.
//!
//! Independent package crate providing real-time CPU, RAM, and FPS monitoring.
//! Exports C-ABI entry points for dynamic loading via `sniffer_pkg_create` / `pkg_create`.

use sniffer_pkg::package::LauncherPackage;
pub use sniffer_pkg::perf_monitor::PerfMonitorPackage;

/// C-ABI entry point for dynamic loading (`libloading` / dlopen).
///
/// # Safety
/// The caller must ensure the returned pointer is wrapped in an `Arc<dyn LauncherPackage>`
/// using [`Arc::from_raw`].
#[unsafe(no_mangle)]
pub extern "C" fn sniffer_pkg_create() -> *mut Box<dyn LauncherPackage> {
    let pkg: Box<dyn LauncherPackage> = Box::new(PerfMonitorPackage::new());
    Box::into_raw(Box::new(pkg))
}

/// Fallback C-ABI entry point matching `pkg_create`.
///
/// # Safety
/// Same as `sniffer_pkg_create`.
#[unsafe(no_mangle)]
pub extern "C" fn pkg_create() -> *mut Box<dyn LauncherPackage> {
    sniffer_pkg_create()
}
