use super::types::{SnifferPackageDescriptorV1, SnifferSlice};
use crate::error::PackageError;
use crate::package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta};
use sniffer_core::vault::DataVault;
use std::sync::Arc;

/// Safe RAII wrapper for dynamic libraries loaded through the C ABI v1 contract.
///
/// Ensures memory isolation by invoking `vtable.destroy(instance)` in the
/// guest library's memory space when the package is unloaded/dropped.
pub struct CApiPackageAdapter {
    descriptor: SnifferPackageDescriptorV1,
    meta: PackageMeta,
}

unsafe impl Send for CApiPackageAdapter {}
unsafe impl Sync for CApiPackageAdapter {}

impl CApiPackageAdapter {
    /// Constructs an adapter after verifying descriptor compatibility and validity.
    ///
    /// # Errors
    /// Returns [`PackageError::DynLoad`] or [`PackageError::Init`] if the descriptor
    /// is invalid or incompatible.
    pub fn new(descriptor: SnifferPackageDescriptorV1) -> Result<Self, PackageError> {
        if !descriptor.header.is_compatible() {
            return Err(PackageError::DynLoad(format!(
                "Incompatible package header: magic={:?}, abi_version={}",
                descriptor.header.magic, descriptor.header.abi_version
            )));
        }

        if descriptor.instance.is_null() {
            return Err(PackageError::DynLoad(
                "Package returned null instance pointer".to_string(),
            ));
        }

        let pkg_id_str = unsafe { descriptor.header.package_id.as_str() }
            .ok_or_else(|| PackageError::DynLoad("Invalid UTF-8 in package_id".to_string()))?;

        let meta = PackageMeta {
            id: Box::leak(pkg_id_str.to_string().into_boxed_str()),
            version: (
                descriptor.header.version_major,
                descriptor.header.version_minor,
                descriptor.header.version_patch,
            ),
            kind: PackageKind::Service,
        };

        Ok(Self { descriptor, meta })
    }

    /// Access the underlying raw descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &SnifferPackageDescriptorV1 {
        &self.descriptor
    }
}

impl Drop for CApiPackageAdapter {
    fn drop(&mut self) {
        if !self.descriptor.instance.is_null() {
            unsafe {
                (self.descriptor.vtable.destroy)(self.descriptor.instance);
            }
            self.descriptor.instance = std::ptr::null_mut();
        }
    }
}

impl LauncherPackage for CApiPackageAdapter {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }

    fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
        let code = unsafe { (self.descriptor.vtable.init)(self.descriptor.instance) };
        if code == 0 {
            Ok(())
        } else {
            Err(PackageError::Init(format!(
                "C ABI init failed with exit code {code}"
            )))
        }
    }

    fn on_update(&self, _vault: &DataVault, dt_secs: f32) {
        unsafe {
            (self.descriptor.vtable.update)(self.descriptor.instance, dt_secs);
        }
    }

    fn on_memory_trim(&self, level: MemoryTrimLevel) {
        let lvl_u32 = match level {
            MemoryTrimLevel::Moderate => 0,
            MemoryTrimLevel::Critical => 1,
            MemoryTrimLevel::Emergency => 2,
        };
        unsafe {
            (self.descriptor.vtable.trim_memory)(self.descriptor.instance, lvl_u32);
        }
    }

    fn on_unload(&mut self) {}

    fn query_service(&self, method: &str, payload: &str) -> Result<String, PackageError> {
        let method_slice = SnifferSlice::from_str(method);
        let payload_slice = SnifferSlice::from_str(payload);
        let mut buf = vec![0u8; 8192];
        let mut out_len = 0usize;

        let code = unsafe {
            (self.descriptor.vtable.method_call)(
                self.descriptor.instance,
                method_slice,
                payload_slice,
                buf.as_mut_ptr(),
                buf.len(),
                &raw mut out_len,
            )
        };

        if code == 0 {
            let res_bytes = &buf[..out_len.min(buf.len())];
            String::from_utf8(res_bytes.to_vec()).map_err(|e| {
                PackageError::Poisoned(format!("invalid UTF-8 in method response: {e}"))
            })
        } else {
            Err(PackageError::UnsupportedMethod(format!(
                "method '{method}' failed with code {code}"
            )))
        }
    }
}
