use super::PackageRegistry;
use super::entries::{ServiceEntry, WidgetEntry, log_error};
use crate::package::{LauncherPackage, WidgetPackage};
use std::sync::Arc;

#[cfg(feature = "dynamic")]
use crate::error::PackageError;
#[cfg(feature = "dynamic")]
use std::path::Path;

impl PackageRegistry {
    /// Register a service package.
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

    /// Load and register a **service** package from a shared library at `path`.
    #[cfg(feature = "dynamic")]
    pub unsafe fn load_service_dynamic(&mut self, path: &Path) -> Result<(), PackageError> {
        let (pkg, handle) = unsafe { crate::loader::load_service_dylib(path) }?;
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
    #[cfg(feature = "dynamic")]
    pub unsafe fn load_widget_dynamic(&mut self, path: &Path) -> Result<(), PackageError> {
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
}
