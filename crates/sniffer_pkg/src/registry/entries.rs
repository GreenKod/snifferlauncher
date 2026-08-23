use crate::package::{LauncherPackage, WidgetPackage};
use std::sync::Arc;

/// Entry for a registered service package.
pub(crate) struct ServiceEntry {
    pub(crate) pkg: Arc<dyn LauncherPackage>,
    /// True if the package panicked on the last `on_update` call.
    pub(crate) faulted: bool,
}

/// Entry for a registered widget package.
pub(crate) struct WidgetEntry {
    pub(crate) pkg: Arc<dyn WidgetPackage>,
    /// Widget ID string used to look up the Taffy layout rect.
    /// Corresponds to `PackageMeta::id`.
    pub(crate) widget_id: &'static str,
    /// Z-index; higher values are drawn on top.
    pub(crate) z_index: i32,
    /// Insertion order (FIFO tiebreaker).
    pub(crate) registration_order: usize,
    /// True if the package panicked on the last `on_render` call.
    pub(crate) faulted: bool,
}

pub(crate) fn log_error(pkg_id: &str, phase: &str, msg: &str) {
    sniffer_core::dev_err!("[sniffer_pkg] '{pkg_id}' {phase} error: {msg}");
}

pub(crate) fn log_panic(pkg_id: &str, phase: &str) {
    sniffer_core::dev_err!(
        "[sniffer_pkg] '{pkg_id}' panicked in {phase}; deactivating package for this frame"
    );
}
