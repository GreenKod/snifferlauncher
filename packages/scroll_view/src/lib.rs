//! `pkg_scroll` — Native GPU ScrollView Widget Package.
//!
//! Independent package crate providing momentum scrolling, boundary elasticity,
//! snap-to-page physics, and overlay rendering.
//! Implements [`WidgetPackage`] and exports C-ABI entry points for dynamic loading.

pub mod constants;
pub mod physics;
pub mod render;
pub mod state;

#[cfg(test)]
mod tests;

use sniffer_core::{math::Rect, render::Renderer, vault::DataVault};
use sniffer_pkg::error::PackageError;
use sniffer_pkg::package::{MemoryTrimLevel, PackageKind, PackageMeta, WidgetPackage};
use state::ScrollState;
use std::sync::{Arc, RwLock};

/// Native GPU widget that hosts a scrollable content area.
pub struct ScrollViewPackage {
    meta: PackageMeta,
    /// All mutable scroll state behind an `RwLock`.
    state: RwLock<ScrollState>,
}

impl ScrollViewPackage {
    /// Create a new scroll view widget with default physics settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: PackageMeta {
                id: "com.sniffer.scroll_view",
                version: (1, 0, 0),
                kind: PackageKind::Widget,
            },
            state: RwLock::new(ScrollState::default()),
        }
    }

    /// Apply an external scroll delta (from the gesture/input layer).
    pub fn apply_scroll_delta(&self, dx: f32, dy: f32, velocity_x: f32, velocity_y: f32) {
        let Ok(mut state) = self.state.write() else {
            return;
        };
        state.scroll_x += dx;
        state.scroll_y += dy;
        if velocity_x.abs() > 0.0 {
            state.velocity_x = velocity_x;
        }
        if velocity_y.abs() > 0.0 {
            state.velocity_y = velocity_y;
        }
    }

    /// Returns the current physical scroll offset `(scroll_x, scroll_y)`.
    #[must_use]
    pub fn scroll_position(&self) -> (f32, f32) {
        let Ok(state) = self.state.read() else {
            return (0.0, 0.0);
        };
        (state.scroll_x, state.scroll_y)
    }

    /// Returns the zero-based active page index.
    #[must_use]
    pub fn current_page(&self) -> i32 {
        self.state.read().map_or(0, |s| s.current_page)
    }
}

impl Default for ScrollViewPackage {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetPackage for ScrollViewPackage {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }

    fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
        Ok(())
    }

    fn on_update(&self, vault: &DataVault, dt_secs: f32) {
        let Ok(mut st) = self.state.write() else {
            return;
        };
        let result = physics::advance_simulation(&mut st, dt_secs);
        // Write the snap event into the vault here so that physics.rs remains
        // a pure, DataVault-free simulation module.
        if let Some((key, value, authority)) = result.snap_event {
            let _ = vault.set(&key, &value, authority);
        }
    }

    fn on_render(&self, renderer: &mut dyn Renderer, layout_rect: Rect, clip_rect: Option<Rect>) {
        let Ok(st) = self.state.read() else {
            return;
        };
        render::render_scroll_view(&st, renderer, layout_rect, clip_rect);
    }

    fn on_memory_trim(&self, _level: MemoryTrimLevel) {}

    fn on_unload(&mut self) {}

    fn apply_descriptor(&self, descriptor_json: &str) -> Result<(), PackageError> {
        let mut st = self
            .state
            .write()
            .map_err(|_| PackageError::Poisoned("ScrollViewPackage::state".into()))?;
        st.apply_descriptor(descriptor_json)
    }
}

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
