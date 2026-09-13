use super::adapter::CApiPackageAdapter;
use super::types::{
    SNIFFER_PKG_ABI_VERSION_1, SNIFFER_PKG_MAGIC, SnifferPackageDescriptorV1, SnifferPackageHeader,
    SnifferPackageVTableV1, SnifferSlice,
};
use crate::package::LauncherPackage;
use sniffer_core::vault::DataVault;
use std::sync::Arc;

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
