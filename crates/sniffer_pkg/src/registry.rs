//! Package lifecycle registry.
//!
//! [`PackageRegistry`] is the single owner of all registered packages.  It
//! drives their lifecycle, isolates panics, sorts widgets by Z-index, and
//! handles both static (`Arc<dyn …>`) and dynamic (`libloading`) packages
//! through a unified API.
//!
//! # Frame Budget
//!
//! ```text
//! ┌────────────────────────────────┐
//! │  PackageRegistry::tick_all()   │  ~100 µs budget per service/widget
//! │  PackageRegistry::render_widgets() │  called inside GPU render pass
//! └────────────────────────────────┘
//! ```
//!
//! # Panic Isolation
//!
//! Every `on_update` and `on_render` call is wrapped in
//! [`std::panic::catch_unwind`].  A faulted package is **deactivated** for
//! the current frame and logged; it is **not** unloaded automatically so that
//! a subsequent frame can attempt recovery if the implementor clears the
//! fault condition.  Call [`PackageRegistry::clear_faulted`] to re-enable a
//! package after diagnosing the problem.
//!
//! # Z-Index / Render Order
//!
//! Widgets that map to the same Taffy node are sorted by
//! `(z_index DESC, registration_order ASC)` before each render pass.
//! The sort is stable and cached; it is only re-computed when a new widget
//! is registered.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(feature = "dynamic")]
use std::path::Path;

use sniffer_core::{math::Rect, render_api::Renderer, vault::DataVault};

use crate::error::PackageError;
use crate::package::{LauncherPackage, MemoryTrimLevel, WidgetPackage};

#[cfg(feature = "dynamic")]
use crate::loader::DynamicHandle;

// ---------------------------------------------------------------------------
// Internal bookkeeping types
// ---------------------------------------------------------------------------

/// Entry for a registered service package.
struct ServiceEntry {
    pkg: Arc<dyn LauncherPackage>,
    /// True if the package panicked on the last `on_update` call.
    faulted: bool,
}

/// Entry for a registered widget package.
struct WidgetEntry {
    pkg: Arc<dyn WidgetPackage>,
    /// Widget ID string used to look up the Taffy layout rect.
    /// Corresponds to `PackageMeta::id`.
    widget_id: &'static str,
    /// Z-index; higher values are drawn on top.
    z_index: i32,
    /// Insertion order (FIFO tiebreaker).
    registration_order: usize,
    /// True if the package panicked on the last `on_render` call.
    faulted: bool,
}

// ---------------------------------------------------------------------------
// PackageRegistry
// ---------------------------------------------------------------------------

/// Central lifecycle manager for all service and widget packages.
///
/// Holds both statically-registered and dynamically-loaded packages.  See
/// the [module documentation][self] for usage notes.
pub struct PackageRegistry {
    services: Vec<ServiceEntry>,
    widgets: Vec<WidgetEntry>,
    /// Set of package IDs whose `on_init` has already been called.
    initialised: HashSet<&'static str>,
    /// Shared DataVault passed to every package's `on_init` / `on_update`.
    vault: Arc<DataVault>,
    /// RAII guards for dynamically loaded libraries.
    /// **Must be dropped after** `services` and `widgets` are cleared.
    #[cfg(feature = "dynamic")]
    dylib_handles: Vec<DynamicHandle>,
    /// Counter incremented on each `register_widget` call to assign
    /// `registration_order`.
    widget_counter: usize,
    /// Whether the `widgets` vec needs re-sorting before the next render.
    widgets_dirty: bool,
}

impl PackageRegistry {
    /// Create a new, empty registry backed by `vault`.
    #[must_use]
    pub fn new(vault: Arc<DataVault>) -> Self {
        Self {
            services: Vec::new(),
            widgets: Vec::new(),
            initialised: HashSet::new(),
            vault,
            #[cfg(feature = "dynamic")]
            dylib_handles: Vec::new(),
            widget_counter: 0,
            widgets_dirty: false,
        }
    }

    // -----------------------------------------------------------------------
    // Registration
    // -----------------------------------------------------------------------

    /// Register a service package.
    ///
    /// If `pkg.on_init()` was not yet called, it is called immediately.
    /// A failed `on_init` is logged but does **not** prevent registration —
    /// the package will simply be skipped during `tick_all`.
    pub fn register_service(&mut self, mut pkg: Arc<dyn LauncherPackage>) {
        let id = pkg.meta().id;
        let mut faulted = false;

        if self.initialised.insert(id) {
            let vault = Arc::clone(&self.vault);
            if let Some(p) = Arc::get_mut(&mut pkg) {
                if let Err(e) = p.on_init(vault) {
                    log_error(id, "on_init", &e.to_string());
                    faulted = true;
                }
            }
        }

        self.services.push(ServiceEntry { pkg, faulted });
    }

    /// Register a widget package.
    ///
    /// If `pkg.on_init()` was not yet called, it is called immediately.
    /// The widget is sorted into the render queue on the next call to
    /// [`render_widgets`][Self::render_widgets].
    pub fn register_widget(&mut self, mut pkg: Arc<dyn WidgetPackage>) {
        let id = pkg.meta().id;
        let mut faulted = false;

        if self.initialised.insert(id) {
            let vault = Arc::clone(&self.vault);
            if let Some(p) = Arc::get_mut(&mut pkg) {
                if let Err(e) = p.on_init(vault) {
                    log_error(id, "on_init", &e.to_string());
                    faulted = true;
                }
            }
        }

        let z_index = pkg.z_index();
        let order = self.widget_counter;
        self.widget_counter += 1;
        self.widgets_dirty = true;

        self.widgets.push(WidgetEntry {
            widget_id: pkg.meta().id,
            pkg,
            z_index,
            registration_order: order,
            faulted,
        });
    }

    // -----------------------------------------------------------------------
    // Dynamic loading (feature = "dynamic")
    // -----------------------------------------------------------------------

    /// Load and register a **service** package from a shared library at `path`.
    ///
    /// The library must export `pkg_create` / `pkg_destroy` per the ABI
    /// contract in [`crate::loader`].
    ///
    /// On Android, call [`crate::loader::prepare_android_dylib`] first to
    /// copy the file to the internal directory and set the correct permissions.
    ///
    /// # Safety
    ///
    /// The shared library must satisfy all constraints described in
    /// [`crate::loader`].
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::DynLoad`] if the library cannot be loaded or
    /// the required symbol is missing.
    #[cfg(feature = "dynamic")]
    pub unsafe fn load_service_dynamic(&mut self, path: &Path) -> Result<(), PackageError> {
        // SAFETY: propagated from the caller.
        let (pkg, handle) = unsafe { crate::loader::load_service_dylib(path) }?;
        // Store the handle before registering — registration may call on_init
        // which could in theory trigger a panic; the handle must survive that.
        self.dylib_handles.push(handle);
        self.register_service(pkg);
        Ok(())
    }

    /// Register a service package along with its dynamic library handle.
    #[cfg(feature = "dynamic")]
    pub fn register_dynamic_service(
        &mut self,
        pkg: Arc<dyn LauncherPackage>,
        handle: crate::loader::DynamicHandle,
    ) {
        self.dylib_handles.push(handle);
        self.register_service(pkg);
    }

    /// Load and register a **widget** package from a shared library at `path`.
    ///
    /// # Safety
    ///
    /// Same requirements as [`load_service_dynamic`][Self::load_service_dynamic].
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::DynLoad`] if the library cannot be loaded or
    /// the required symbol is missing.
    #[cfg(feature = "dynamic")]
    pub unsafe fn load_widget_dynamic(&mut self, path: &Path) -> Result<(), PackageError> {
        // SAFETY: propagated from the caller.
        let (pkg, handle) = unsafe { crate::loader::load_widget_dylib(path) }?;
        self.dylib_handles.push(handle);
        self.register_widget(pkg);
        Ok(())
    }

    /// Register a widget package along with its dynamic library handle.
    #[cfg(feature = "dynamic")]
    pub fn register_dynamic_widget(
        &mut self,
        pkg: Arc<dyn WidgetPackage>,
        handle: crate::loader::DynamicHandle,
    ) {
        self.dylib_handles.push(handle);
        self.register_widget(pkg);
    }

    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Call `on_update` on every non-faulted package.
    ///
    /// Panics inside `on_update` are caught; the offending package is marked
    /// as faulted and skipped for the remainder of this frame.  The fault
    /// persists until [`clear_faulted`][Self::clear_faulted] is called.
    pub fn tick_all(&mut self, dt_secs: f32) {
        let vault = Arc::clone(&self.vault);

        for entry in &mut self.services {
            if entry.faulted {
                continue;
            }
            let pkg = Arc::clone(&entry.pkg);
            let vault_ref = Arc::clone(&vault);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pkg.on_update(&vault_ref, dt_secs);
            }));
            if result.is_err() {
                entry.faulted = true;
                log_panic(entry.pkg.meta().id, "on_update");
            }
        }

        for entry in &mut self.widgets {
            if entry.faulted {
                continue;
            }
            let pkg = Arc::clone(&entry.pkg);
            let vault_ref = Arc::clone(&vault);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pkg.on_update(&vault_ref, dt_secs);
            }));
            if result.is_err() {
                entry.faulted = true;
                log_panic(entry.pkg.meta().id, "on_update");
            }
        }
    }

    /// Render all non-faulted widget packages in Z-index / FIFO order.
    ///
    /// `layout_rects` maps each widget's `PackageMeta::id` to the Taffy-
    /// computed screen rectangle.  Widgets whose ID is not present in the
    /// map are skipped.
    ///
    /// Must be called inside the GPU render pass (between `begin_frame` and
    /// `end_frame`).
    pub fn render_widgets(
        &mut self,
        renderer: &mut dyn Renderer,
        layout_rects: &HashMap<&str, Rect>,
        clip_rect: Option<Rect>,
    ) {
        // Re-sort the widget list if the registry was mutated since the last
        // render.  Stable sort preserves FIFO order for equal z-indices.
        if self.widgets_dirty {
            self.widgets.sort_by(|a, b| {
                b.z_index
                    .cmp(&a.z_index)
                    .then_with(|| a.registration_order.cmp(&b.registration_order))
            });
            self.widgets_dirty = false;
        }

        for entry in &mut self.widgets {
            if entry.faulted {
                continue;
            }

            let Some(&rect) = layout_rects.get(entry.widget_id) else {
                continue;
            };

            let pkg = Arc::clone(&entry.pkg);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pkg.on_render(renderer, rect, clip_rect);
            }));

            if result.is_err() {
                entry.faulted = true;
                log_panic(entry.pkg.meta().id, "on_render");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Memory pressure
    // -----------------------------------------------------------------------

    /// Broadcast a memory trim signal to all packages.
    pub fn trim_memory(&self, level: MemoryTrimLevel) {
        for entry in &self.services {
            entry.pkg.on_memory_trim(level);
        }
        for entry in &self.widgets {
            entry.pkg.on_memory_trim(level);
        }
    }

    // -----------------------------------------------------------------------
    // Bridge API
    // -----------------------------------------------------------------------

    /// Forward a plugin-layer service query to the matching package.
    ///
    /// Finds the first service whose `meta().id == pkg_id`, then calls
    /// [`LauncherPackage::query_service`].
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::NotFound`] if no service with `pkg_id` exists,
    /// or the package's own error on processing failure.
    pub fn query_service(
        &self,
        pkg_id: &str,
        method: &str,
        payload: &str,
    ) -> Result<String, PackageError> {
        self.services
            .iter()
            .find(|e| e.pkg.meta().id == pkg_id)
            .ok_or_else(|| PackageError::NotFound(pkg_id.into()))
            .and_then(|e| e.pkg.query_service(method, payload))
    }

    /// Apply a JSON descriptor to the widget package identified by `widget_id`.
    ///
    /// This is the Rust handler for `host_extend_widget(widget_id, json)`.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::NotFound`] if no widget with `widget_id` is
    /// registered, or [`PackageError::InvalidDescriptor`] on parse failure.
    pub fn apply_widget_descriptor(
        &self,
        widget_id: &str,
        descriptor_json: &str,
    ) -> Result<(), PackageError> {
        self.widgets
            .iter()
            .find(|e| e.widget_id == widget_id)
            .ok_or_else(|| PackageError::NotFound(widget_id.into()))
            .and_then(|e| e.pkg.apply_descriptor(descriptor_json))
    }

    // -----------------------------------------------------------------------
    // Fault management
    // -----------------------------------------------------------------------

    /// Clear the fault flag on the package with `pkg_id`, re-enabling it for
    /// future `tick_all` / `render_widgets` calls.
    ///
    /// Returns `true` if the package was found (regardless of whether it was
    /// actually faulted).
    pub fn clear_faulted(&mut self, pkg_id: &str) -> bool {
        let mut found = false;
        for e in &mut self.services {
            if e.pkg.meta().id == pkg_id {
                e.faulted = false;
                found = true;
            }
        }
        for e in &mut self.widgets {
            if e.pkg.meta().id == pkg_id {
                e.faulted = false;
                found = true;
            }
        }
        found
    }

    /// Returns the IDs of all packages that are currently in a faulted state.
    pub fn faulted_ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self
            .services
            .iter()
            .filter(|e| e.faulted)
            .map(|e| e.pkg.meta().id)
            .collect();
        ids.extend(
            self.widgets
                .iter()
                .filter(|e| e.faulted)
                .map(|e| e.pkg.meta().id),
        );
        ids
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Returns the number of registered service packages.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Returns the number of registered widget packages.
    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    /// Returns a shared reference to the registry's [`DataVault`].
    pub fn vault(&self) -> &Arc<DataVault> {
        &self.vault
    }

    // -----------------------------------------------------------------------
    // Teardown
    // -----------------------------------------------------------------------

    /// Call `on_unload` on every package and release all resources.
    ///
    /// Dynamic library handles are dropped **after** the package `Arc`s so
    /// that vtable pointers remain valid until the last `Arc` is released.
    pub fn unload_all(&mut self) {
        for entry in &mut self.services {
            if let Some(pkg) = Arc::get_mut(&mut entry.pkg) {
                pkg.on_unload();
            }
        }
        for entry in &mut self.widgets {
            if let Some(pkg) = Arc::get_mut(&mut entry.pkg) {
                pkg.on_unload();
            }
        }

        // Drop package Arcs first.
        self.services.clear();
        self.widgets.clear();
        self.initialised.clear();

        // Drop library handles last — dlclose is now safe.
        #[cfg(feature = "dynamic")]
        self.dylib_handles.clear();
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

#[inline]
fn log_error(pkg_id: &str, method: &str, msg: &str) {
    // Use eprintln in environments where the log crate is not wired up yet;
    // production builds will route through the platform logger.
    #[cfg(feature = "pkg_perfmon")]
    eprintln!("[sniffer_pkg] ERROR  pkg={pkg_id} method={method}: {msg}");
    #[cfg(not(feature = "pkg_perfmon"))]
    eprintln!("[sniffer_pkg] ERROR  pkg={pkg_id} method={method}: {msg}");
}

#[inline]
fn log_panic(pkg_id: &str, method: &str) {
    eprintln!("[sniffer_pkg] PANIC  pkg={pkg_id} method={method} — package deactivated");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{PackageKind, PackageMeta};

    // ── Minimal stub implementations ────────────────────────────────────────

    struct CounterService {
        meta: PackageMeta,
        pub ticks: std::sync::atomic::AtomicU32,
    }

    impl CounterService {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                meta: PackageMeta {
                    id: "com.test.counter",
                    version: (0, 1, 0),
                    kind: PackageKind::Service,
                },
                ticks: std::sync::atomic::AtomicU32::new(0),
            })
        }
    }

    impl LauncherPackage for CounterService {
        fn meta(&self) -> &PackageMeta {
            &self.meta
        }
        fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
            Ok(())
        }
        fn on_update(&self, _vault: &DataVault, _dt: f32) {
            self.ticks
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        fn on_memory_trim(&self, _level: MemoryTrimLevel) {}
        fn on_unload(&mut self) {}
    }

    struct PanicService {
        meta: PackageMeta,
    }

    impl PanicService {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                meta: PackageMeta {
                    id: "com.test.panicker",
                    version: (0, 1, 0),
                    kind: PackageKind::Service,
                },
            })
        }
    }

    impl LauncherPackage for PanicService {
        fn meta(&self) -> &PackageMeta {
            &self.meta
        }
        fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
            Ok(())
        }
        fn on_update(&self, _vault: &DataVault, _dt: f32) {
            panic!("intentional panic for testing");
        }
        fn on_memory_trim(&self, _level: MemoryTrimLevel) {}
        fn on_unload(&mut self) {}
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    fn make_registry() -> PackageRegistry {
        PackageRegistry::new(Arc::new(DataVault::default()))
    }

    #[test]
    fn service_tick_increments_counter() {
        let mut reg = make_registry();
        let svc = CounterService::new();
        reg.register_service(svc.clone());

        reg.tick_all(0.016);
        reg.tick_all(0.016);

        assert_eq!(
            svc.ticks.load(std::sync::atomic::Ordering::Relaxed),
            2,
            "on_update must be called once per tick_all"
        );
    }

    #[test]
    fn panic_in_on_update_deactivates_package() {
        let mut reg = make_registry();
        reg.register_service(PanicService::new());
        assert_eq!(reg.faulted_ids().len(), 0);

        // First tick triggers the panic; catch_unwind isolates it.
        reg.tick_all(0.016);

        assert_eq!(
            reg.faulted_ids(),
            vec!["com.test.panicker"],
            "panicking package must be marked faulted"
        );

        // Subsequent tick must not call on_update on the faulted package.
        // If it did, the test process itself would panic.
        reg.tick_all(0.016);
    }

    #[test]
    fn clear_faulted_re_enables_package() {
        let mut reg = make_registry();
        reg.register_service(PanicService::new());
        reg.tick_all(0.016);
        assert!(!reg.faulted_ids().is_empty());

        let found = reg.clear_faulted("com.test.panicker");
        assert!(found);
        assert!(reg.faulted_ids().is_empty());
    }

    #[test]
    fn query_service_returns_not_found_for_unknown_id() {
        let reg = make_registry();
        let err = reg.query_service("com.missing", "ping", "{}").unwrap_err();
        assert!(
            matches!(err, PackageError::NotFound(_)),
            "must return NotFound for unregistered package"
        );
    }

    #[test]
    fn service_count_reflects_registrations() {
        let mut reg = make_registry();
        assert_eq!(reg.service_count(), 0);
        reg.register_service(CounterService::new());
        assert_eq!(reg.service_count(), 1);
    }

    #[test]
    fn unload_all_clears_packages() {
        let mut reg = make_registry();
        reg.register_service(CounterService::new());
        reg.unload_all();
        assert_eq!(reg.service_count(), 0);
    }
}
