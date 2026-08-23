use super::PackageRegistry;
use super::entries::log_panic;
use crate::error::PackageError;
use crate::package::MemoryTrimLevel;
use sniffer_core::{math::Rect, render_api::Renderer};
use std::collections::HashMap;
use std::sync::Arc;

impl PackageRegistry {
    /// Call `on_update` on every non-faulted package.
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
    pub fn render_widgets(
        &mut self,
        renderer: &mut dyn Renderer,
        layout_rects: &HashMap<&str, Rect>,
        clip_rect: Option<Rect>,
    ) {
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

    /// Broadcast a memory trim signal to all packages.
    pub fn trim_memory(&self, level: MemoryTrimLevel) {
        for entry in &self.services {
            entry.pkg.on_memory_trim(level);
        }
        for entry in &self.widgets {
            entry.pkg.on_memory_trim(level);
        }
    }

    /// Forward a plugin-layer service query to the matching package.
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

    /// Clear the fault flag on the package with `pkg_id`.
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

    /// Call `on_unload` on every package and release all resources.
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

        self.services.clear();
        self.widgets.clear();
        self.initialised.clear();

        #[cfg(feature = "dynamic")]
        self.dylib_handles.clear();
    }
}
