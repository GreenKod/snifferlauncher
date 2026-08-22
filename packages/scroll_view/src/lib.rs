//! `pkg_scroll` — Native GPU ScrollView Widget Package.
//!
//! Independent package crate providing momentum scrolling, boundary elasticity,
//! snap-to-page physics, and overlay rendering.
//! Exports C-ABI entry points for dynamic loading via `sniffer_widget_create` / `pkg_create_widget`.

use sniffer_pkg::package::WidgetPackage;
pub use sniffer_pkg::scroll_view::ScrollViewPackage;

/// C-ABI entry point for dynamic loading (`libloading` / dlopen).
///
/// # Safety
/// The caller must ensure the returned pointer is wrapped in an `Arc<dyn WidgetPackage>`
/// using [`Arc::from_raw`].
#[unsafe(no_mangle)]
pub extern "C" fn sniffer_widget_create() -> *mut Box<dyn WidgetPackage> {
    let pkg: Box<dyn WidgetPackage> = Box::new(ScrollViewPackage::new());
    Box::into_raw(Box::new(pkg))
}

/// Fallback C-ABI entry point matching `pkg_create_widget`.
///
/// # Safety
/// Same as `sniffer_widget_create`.
#[unsafe(no_mangle)]
pub extern "C" fn pkg_create_widget() -> *mut Box<dyn WidgetPackage> {
    sniffer_widget_create()
}
