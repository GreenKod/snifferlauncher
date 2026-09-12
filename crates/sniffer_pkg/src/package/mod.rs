//! Core traits for the package layer.
//!
//! Two complementary traits model the two kinds of packages:
//!
//! | Trait                | Purpose                                      |
//! |----------------------|----------------------------------------------|
//! | [`LauncherPackage`]  | Service / runtime provider (no GPU access)   |
//! | [`WidgetPackage`]    | Native GPU widget (Taffy + Renderer access)  |
//!
//! Both share [`PackageMeta`] for identification and [`MemoryTrimLevel`] for
//! back-pressure signals from the Android / desktop memory subsystem.

pub mod c_abi;

use crate::error::PackageError;
use sniffer_core::{
    math::{Rect, Size},
    render::Renderer,
    vault::DataVault,
};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Package metadata
// ---------------------------------------------------------------------------

/// Functional category of a package.
///
/// Used by [`PackageRegistry`][crate::registry::PackageRegistry] to route
/// lifecycle calls correctly and to decide which render slots to allocate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageKind {
    /// Runtime, system-metric, database, or other headless service.
    Service,
    /// Native GPU widget rendered directly into the Taffy layout tree.
    Widget,
    /// Provides both a background service **and** a rendered widget.
    Hybrid,
}

/// Immutable identity and version information for a package.
///
/// Stored as `'static` string slices so that no heap allocation is needed
/// when querying metadata in the hot render path.
#[derive(Clone, Debug)]
pub struct PackageMeta {
    /// Reverse-DNS identifier — must be globally unique.
    /// Example: `"com.sniffer.perf"`.
    pub id: &'static str,
    /// Semantic version `(major, minor, patch)`.
    pub version: (u8, u8, u8),
    /// Functional category of this package.
    pub kind: PackageKind,
}

// ---------------------------------------------------------------------------
// Memory trim levels
// ---------------------------------------------------------------------------

/// Back-pressure signal passed to [`LauncherPackage::on_memory_trim`] and
/// [`WidgetPackage::on_memory_trim`].
///
/// Maps directly to Android's `ComponentCallbacks2.onTrimMemory` severity
/// levels and is also supported on desktop (simulated from OS events).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTrimLevel {
    /// Application went to background — optionally release non-critical caches.
    Moderate,
    /// System is under memory pressure — release all non-essential memory.
    Critical,
    /// Severe memory pressure — reduce to the absolute minimum working set.
    Emergency,
}

// ---------------------------------------------------------------------------
// LauncherPackage
// ---------------------------------------------------------------------------

/// A headless package that provides services, runtimes, or system metrics.
///
/// `LauncherPackage` implementations **must not** call any `Renderer` methods;
/// all GPU interaction belongs in [`WidgetPackage`].  Data is shared with
/// the rest of the system via [`DataVault`].
///
/// # Thread Safety
///
/// `Send + Sync` is required.  Heavy or blocking work (network, file I/O,
/// system calls) **must** be offloaded to a dedicated background thread
/// spawned inside [`on_init`][Self::on_init]; the `on_update` call budget
/// is approximately 100 µs per frame at 60 fps.
///
/// # Dynamic ABI
///
/// When compiled as a `cdylib` for runtime loading, the concrete type
/// implementing this trait must also export:
///
/// ```ignore
/// #[no_mangle]
/// pub unsafe extern "C" fn pkg_create() -> *mut Box<dyn LauncherPackage> { … }
///
/// #[no_mangle]
/// pub unsafe extern "C" fn pkg_destroy(ptr: *mut Box<dyn LauncherPackage>) { … }
/// ```
///
/// See [`crate::loader`] for the host-side loading contract.
pub trait LauncherPackage: Send + Sync {
    /// Returns the static identity of this package.
    fn meta(&self) -> &PackageMeta;

    /// Called once when the package is first registered.
    ///
    /// Spawn any required background threads here.  The `vault` handle may be
    /// cloned and moved into those threads for reactive writes.
    ///
    /// # Errors
    ///
    /// Return [`PackageError::Init`] if start-up cannot complete successfully.
    fn on_init(&mut self, vault: Arc<DataVault>) -> Result<(), PackageError>;

    /// Called once per render frame on the **main thread**.
    ///
    /// Keep this method **non-blocking**.  If you need to push data to the
    /// vault at frame rate, prefer an `AtomicCell` / channel approach where
    /// the background thread writes and this method just flushes the buffer.
    fn on_update(&self, vault: &DataVault, dt_secs: f32);

    /// Called when the OS signals memory pressure.
    ///
    /// Release optional caches, reduce polling frequency, or suspend
    /// background threads according to `level`.
    fn on_memory_trim(&self, level: MemoryTrimLevel);

    /// Called before the package is removed from the registry.
    ///
    /// Stop background threads, close file handles, and release all
    /// resources.  After this call the registry will drop the `Arc`.
    fn on_unload(&mut self);

    /// Handles a named method call from the plugin bridge
    /// (`host_pkg_query(id, method, payload_json)`).
    ///
    /// `payload` is a JSON string; the response must also be a JSON string.
    ///
    /// # Errors
    ///
    /// Return [`PackageError::UnsupportedMethod`] for unknown methods, or a
    /// domain-specific error variant if processing fails.
    fn query_service(&self, method: &str, payload: &str) -> Result<String, PackageError> {
        let _ = (method, payload);
        Err(PackageError::UnsupportedMethod(method.into()))
    }
}

// ---------------------------------------------------------------------------
// WidgetPackage
// ---------------------------------------------------------------------------

/// A native GPU widget that participates directly in the render pipeline.
///
/// The registry calls lifecycle methods in this order each frame:
///
/// ```text
/// on_update()  →  [Taffy layout pass]  →  on_render()
/// ```
///
/// Widget packages have exclusive access to the [`Renderer`] during their
/// `on_render` call.  The renderer is **not** available during `on_update` or
/// any background thread.
///
/// # Render Order
///
/// When multiple widgets occupy the same Taffy node the registry sorts them
/// by `(z_index DESC, registration_order ASC)` — i.e. higher z-index is
/// drawn on top; equal z-index uses insertion order (FIFO).
///
/// # Dynamic ABI
///
/// Same ABI requirement as [`LauncherPackage`], but the exported symbol is
/// `pkg_create_widget` / `pkg_destroy_widget`.
pub trait WidgetPackage: Send + Sync {
    /// Returns the static identity of this package.
    fn meta(&self) -> &PackageMeta;

    /// Z-index used to sort widgets that occupy the same Taffy node.
    /// Default is `0`.  Higher values are drawn on top.
    fn z_index(&self) -> i32 {
        0
    }

    /// Called once when the package is registered.
    ///
    /// # Errors
    ///
    /// Return [`PackageError::Init`] if the widget cannot initialise.
    fn on_init(&mut self, vault: Arc<DataVault>) -> Result<(), PackageError>;

    /// Per-frame update on the **main thread** before the render pass.
    fn on_update(&self, vault: &DataVault, dt_secs: f32);

    /// Called after the Taffy layout pass, inside the GPU render pass.
    ///
    /// `layout_rect` is the Taffy-computed screen rectangle for this widget's
    /// node.  `clip_rect` is the nearest ancestor clip region, if any.
    ///
    /// Push/pop clip rects and transforms symmetrically.
    fn on_render(&self, renderer: &mut dyn Renderer, layout_rect: Rect, clip_rect: Option<Rect>);

    /// Called when the OS signals memory pressure.
    ///
    /// On [`MemoryTrimLevel::Critical`] or higher, evict GPU texture caches
    /// via the renderer's eviction API if available.
    fn on_memory_trim(&self, level: MemoryTrimLevel);

    /// Called before the package is removed from the registry.
    fn on_unload(&mut self);

    /// Applies a JSON descriptor sent from the plugin layer via
    /// `host_extend_widget(widget_id, descriptor_json)`.
    ///
    /// Implementations should atomically update their internal configuration
    /// so the next `on_render` call reflects the new values.
    ///
    /// # Errors
    ///
    /// Return [`PackageError::InvalidDescriptor`] if the JSON cannot be
    /// parsed or contains out-of-range values.
    fn apply_descriptor(&self, descriptor_json: &str) -> Result<(), PackageError> {
        let _ = descriptor_json;
        Ok(())
    }

    /// Reports the natural content size for Taffy's measure callback.
    ///
    /// `available` is the space offered by the parent node.  Return the
    /// preferred size; Taffy will honour flex constraints on top.
    fn measure_content(&self, available: Size) -> Size {
        available
    }
}
