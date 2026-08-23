//! Package lifecycle registry.
//!
//! [`PackageRegistry`] is the single owner of all registered packages. It
//! drives their lifecycle, isolates panics, sorts widgets by Z-index, and
//! handles both static (`Arc<dyn …>`) and dynamic (`libloading`) packages
//! through a unified API.

pub(crate) mod entries;
pub mod lifecycle;
pub mod registration;

#[cfg(test)]
mod tests;

use entries::{ServiceEntry, WidgetEntry};
use sniffer_core::vault::DataVault;
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(feature = "dynamic")]
use crate::loader::DynamicHandle;

/// Central lifecycle manager for all service and widget packages.
///
/// Holds both statically-registered and dynamically-loaded packages.
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
}
