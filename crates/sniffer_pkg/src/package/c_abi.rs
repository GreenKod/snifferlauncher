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

#[cfg(test)]
mod tests {
    use super::*;

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
}
