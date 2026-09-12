//! C ABI v1 data structures for cross-toolchain dynamic packages.
//! See `crates/sniffer_pkg/docs/adr_001_c_abi_package_contract.md` for full specification.

/// Standard magic identifier for Sniffer native packages: b"SNIF".
pub const SNIFFER_PKG_MAGIC: [u8; 4] = [0x53, 0x4E, 0x49, 0x46];

/// Current C ABI version.
pub const SNIFFER_PKG_ABI_VERSION_1: u32 = 1;

/// Standard borrowed slice across the C boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnifferSlice {
    pub ptr: *const u8,
    pub len: usize,
}

impl SnifferSlice {
    /// Create a new `SnifferSlice` from a byte slice.
    #[must_use]
    pub const fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    /// Create a new `SnifferSlice` from a string slice.
    #[must_use]
    pub const fn from_str(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Convert to a Rust string slice if valid UTF-8.
    ///
    /// # Safety
    /// Caller must ensure `ptr` is valid for reads up to `len` bytes.
    #[must_use]
    pub unsafe fn as_str<'a>(&self) -> Option<&'a str> {
        if self.ptr.is_null() {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
        std::str::from_utf8(slice).ok()
    }
}

/// Package identification and binary compatibility header.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SnifferPackageHeader {
    /// Magic signature, must match [`SNIFFER_PKG_MAGIC`].
    pub magic: [u8; 4],
    /// ABI version, must match [`SNIFFER_PKG_ABI_VERSION_1`].
    pub abi_version: u32,
    /// Unique package ID slice (e.g. "com.sniffer.perf").
    pub package_id: SnifferSlice,
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    pub reserved: u8,
}

impl SnifferPackageHeader {
    /// Validates magic bytes and ABI version.
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        self.magic == SNIFFER_PKG_MAGIC && self.abi_version == SNIFFER_PKG_ABI_VERSION_1
    }
}

/// Dynamic function pointer table (VTable) for lifecycle and inter-process queries.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SnifferPackageVTableV1 {
    /// Initializes package resources. Returns 0 on success, negative error code on failure.
    pub init: unsafe extern "C" fn(ctx: *mut std::ffi::c_void) -> i32,

    /// Per-frame update call.
    pub update: unsafe extern "C" fn(ctx: *mut std::ffi::c_void, dt_secs: f32),

    /// Memory pressure signal (0=Moderate, 1=Critical, 2=Emergency).
    pub trim_memory: unsafe extern "C" fn(ctx: *mut std::ffi::c_void, level: u32),

    /// Handles named queries from the host or other plugins.
    pub method_call: unsafe extern "C" fn(
        ctx: *mut std::ffi::c_void,
        method: SnifferSlice,
        payload: SnifferSlice,
        out_buf: *mut u8,
        out_cap: usize,
        out_len: *mut usize,
    ) -> i32,

    /// Destroys and frees the instance using the guest package allocator.
    pub destroy: unsafe extern "C" fn(ctx: *mut std::ffi::c_void),
}

/// Top-level descriptor returned by dynamic package entry point `sniffer_package_v1_create`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SnifferPackageDescriptorV1 {
    pub header: SnifferPackageHeader,
    pub vtable: SnifferPackageVTableV1,
    pub instance: *mut std::ffi::c_void,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    static mut MOCK_DESTROYED: bool = false;
    static mut MOCK_INITIALIZED: bool = false;

    unsafe extern "C" fn mock_init(_ctx: *mut std::ffi::c_void) -> i32 {
        unsafe { MOCK_INITIALIZED = true };
        0
    }

    unsafe extern "C" fn mock_update(_ctx: *mut std::ffi::c_void, _dt: f32) {}
    unsafe extern "C" fn mock_trim(_ctx: *mut std::ffi::c_void, _level: u32) {}
    unsafe extern "C" fn mock_method(
        _ctx: *mut std::ffi::c_void,
        _method: SnifferSlice,
        _payload: SnifferSlice,
        out_buf: *mut u8,
        out_cap: usize,
        out_len: *mut usize,
    ) -> i32 {
        let reply = b"{\"status\":\"ok\"}";
        if out_cap >= reply.len() {
            unsafe {
                std::ptr::copy_nonoverlapping(reply.as_ptr(), out_buf, reply.len());
                *out_len = reply.len();
            }
            0
        } else {
            -1
        }
    }

    unsafe extern "C" fn mock_destroy(_ctx: *mut std::ffi::c_void) {
        unsafe { MOCK_DESTROYED = true };
    }

    #[test]
    fn test_sniffer_slice_from_str() {
        let text = "hello sniffer";
        let slice = SnifferSlice::from_str(text);
        assert_eq!(slice.len, 13);
        unsafe {
            assert_eq!(slice.as_str(), Some("hello sniffer"));
        }
    }

    #[test]
    fn test_header_compatibility() {
        let header = SnifferPackageHeader {
            magic: SNIFFER_PKG_MAGIC,
            abi_version: SNIFFER_PKG_ABI_VERSION_1,
            package_id: SnifferSlice::from_str("com.test.pkg"),
            version_major: 1,
            version_minor: 0,
            version_patch: 0,
            reserved: 0,
        };
        assert!(header.is_compatible());

        let corrupted = SnifferPackageHeader {
            magic: [0, 0, 0, 0],
            ..header
        };
        assert!(!corrupted.is_compatible());
    }

    #[test]
    fn test_c_abi_adapter_lifecycle_and_destroy() {
        let header = SnifferPackageHeader {
            magic: SNIFFER_PKG_MAGIC,
            abi_version: SNIFFER_PKG_ABI_VERSION_1,
            package_id: SnifferSlice::from_str("com.test.adapter"),
            version_major: 1,
            version_minor: 2,
            version_patch: 3,
            reserved: 0,
        };

        let vtable = SnifferPackageVTableV1 {
            init: mock_init,
            update: mock_update,
            trim_memory: mock_trim,
            method_call: mock_method,
            destroy: mock_destroy,
        };

        // Dummy non-null instance pointer
        let dummy_instance = 0x1234 as *mut std::ffi::c_void;
        let descriptor = SnifferPackageDescriptorV1 {
            header,
            vtable,
            instance: dummy_instance,
        };

        unsafe {
            MOCK_DESTROYED = false;
            MOCK_INITIALIZED = false;
        }

        {
            let mut adapter = CApiPackageAdapter::new(descriptor).expect("valid descriptor");
            assert_eq!(adapter.meta().id, "com.test.adapter");
            assert_eq!(adapter.meta().version, (1, 2, 3));

            // Test query
            let res = adapter.query_service("ping", "{}").expect("query ok");
            assert_eq!(res, "{\"status\":\"ok\"}");

            // Test init
            let vault = Arc::new(DataVault::new(None));
            adapter.on_init(vault).expect("init ok");
            unsafe {
                assert!(MOCK_INITIALIZED);
                assert!(!MOCK_DESTROYED);
            }
        }

        // After dropping adapter, mock_destroy must have been invoked!
        unsafe {
            assert!(MOCK_DESTROYED);
        }
    }

    #[test]
    fn test_c_abi_adapter_rejects_incompatible_magic_and_version() {
        let bad_header = SnifferPackageHeader {
            magic: *b"BAD!",
            abi_version: 99,
            package_id: SnifferSlice::from_str("com.test.bad"),
            version_major: 1,
            version_minor: 0,
            version_patch: 0,
            reserved: 0,
        };
        let vtable = SnifferPackageVTableV1 {
            init: mock_init,
            update: mock_update,
            trim_memory: mock_trim,
            method_call: mock_method,
            destroy: mock_destroy,
        };
        let descriptor = SnifferPackageDescriptorV1 {
            header: bad_header,
            vtable,
            instance: 0x1234 as *mut std::ffi::c_void,
        };

        let err = CApiPackageAdapter::new(descriptor);
        assert!(err.is_err());
    }
}
