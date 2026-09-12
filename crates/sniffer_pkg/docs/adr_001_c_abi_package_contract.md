# ADR 001: Stable Versioned C ABI Contract for Dynamic Native Packages

**Status:** Accepted / Specification  
**Date:** 2026-09-11  
**Area:** `crates/sniffer_pkg`  
**Authors:** SnifferLauncher Core Team  

---

## 1. Context and Problem Statement

`sniffer_pkg` provides a modular extensibility mechanism allowing the launcher to load native compiled packages (services and GPU-rendered widgets) at runtime via dynamic shared libraries (`.so` on Linux/Android, `.dll` on Windows, `.dylib` on macOS).

In the initial implementation (`crates/sniffer_pkg/src/loader/dylib.rs`), packages are instantiated via raw Rust trait objects:
```rust
type ServiceCreateFn = unsafe extern "C" fn() -> *mut Box<dyn LauncherPackage>;
```

### Architectural & Security Hazards
1. **Unstable Rust ABI:** Rust does not possess a stable Application Binary Interface (ABI). The memory layout of `dyn Trait` fat pointers (data pointer + vtable pointer) and struct representations varies across Rust compiler versions (`rustc`), compiler flags, target triples, and optimization profiles (`opt-level`).
2. **Cross-Boundary Heap Deallocation Hazards:** Calling `*Box::from_raw(raw)` in the host process assumes that the host and the dynamic library use the exact same memory allocator. If a third-party package is compiled with a different allocator (or a different glibc/musl/jemalloc runtime), deallocating the guest object in the host triggers memory corruption, undefined behavior (UB), or segmentation faults.
3. **Absence of Version Handshake & Capability Gating:** There was no runtime header handshake verifying whether the dynamic library was built for the correct launcher API version before executing native functions.
4. **Supply Chain & Execution Boundary:** Executing untrusted native shared libraries without signature, checksum, or sandboxing presents an arbitrary code execution risk.

---

## 2. Decision: Versioned C ABI Architecture

We establish a **Stable Versioned C ABI Contract (v1)** for all native dynamically loaded packages.

```
+----------------------------------------------------------------+
|                        Host Application                        |
|   (sniffer_pkg Loader, Registry, Lifecycle Management)         |
+----------------------------------------------------------------+
                               |
                   [Stable C ABI Boundary]
      - 32-bit Magic (0x534E4946 "SNIF")
      - Major API Version Handshake
      - Pod Types (repr(C)) & Slices (ptr, len)
      - Function Pointer Table (VTable)
      - Explicit Guest Deallocator (destroy callback)
                               |
+----------------------------------------------------------------+
|                    Dynamic Shared Library                      |
|           (Third-party package / Native Widget)                |
+----------------------------------------------------------------+
```

---

## 3. Specification

### 3.1 Magic Number & Header
Every dynamic library exports an initialization symbol:
```c
extern "C" SnifferPackageDescriptorV1* sniffer_package_v1_create(void);
```

The descriptor begins with a standard header:
```rust
#[repr(C)]
pub struct SnifferSlice {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
pub struct SnifferPackageHeader {
    pub magic: [u8; 4],          // Must be [0x53, 0x4E, 0x49, 0x46] ("SNIF")
    pub abi_version: u32,        // Must be 1 for v1
    pub package_id: SnifferSlice,// Unique reverse-DNS identifier
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    pub reserved: u8,
}
```

### 3.2 VTable and Opaque Instance
Interaction occurs strictly through function pointers receiving an opaque instance pointer (`*mut c_void`):

```rust
#[repr(C)]
pub struct SnifferPackageVTableV1 {
    /// Initializes package resources. Returns 0 on success, negative on error.
    pub init: unsafe extern "C" fn(ctx: *mut std::ffi::c_void) -> i32,
    
    /// Per-frame update step.
    pub update: unsafe extern "C" fn(ctx: *mut std::ffi::c_void, dt_secs: f32),
    
    /// Memory trim pressure signal (0=Moderate, 1=Critical, 2=Emergency).
    pub trim_memory: unsafe extern "C" fn(ctx: *mut std::ffi::c_void, level: u32),
    
    /// Invokes a named method query.
    pub method_call: unsafe extern "C" fn(
        ctx: *mut std::ffi::c_void,
        method: SnifferSlice,
        payload: SnifferSlice,
        out_buf: *mut u8,
        out_cap: usize,
        out_len: *mut usize,
    ) -> i32,
    
    /// Destroys and deallocates the instance using the GUEST allocator.
    pub destroy: unsafe extern "C" fn(ctx: *mut std::ffi::c_void),
}
```

### 3.3 Lifecycle & Memory Ownership Rules
1. **Allocation Ownership:** Any object allocated by the dynamic package must be freed by the dynamic package. The host **never** calls `free()`, `Box::from_raw()`, or deallocates guest pointers directly.
2. **Destruction Guarantee:** When unloading a package, the host invokes `vtable.destroy(instance)`. The guest library casts `instance` back to its concrete type and drops it.
3. **Strings and Payloads:** Strings and byte buffers cross the boundary as `SnifferSlice` (`ptr + len`). The borrowing party must not retain the slice beyond the duration of the call without making a private copy.
4. **Header Verification:** During loading, the host reads `header.magic` and `header.abi_version`. If magic does not equal `"SNIF"` or `abi_version != 1`, the library is rejected before calling any lifecycle function.

---

## 4. Package Trust and Integrity Model

1. **Manifest Integrity:** Packages distributed in the ecosystem must provide a `package.toml` manifest declaring:
   - `id`: Reverse-DNS identifier matching `header.package_id`.
   - `version`: Matching `header.version_*`.
   - `sha256`: Cryptographic digest of the compiled binary (`.so/.dll`).
2. **Safe Discovery Paths:** Libraries are only loaded from the designated user package directory (e.g., `~/.sniffer/packages/` or Android application private storage). Loading from world-writable directories (`/tmp`, `/sdcard`) is strictly disallowed.

---

## 5. Consequences

### Positive
- Full toolchain independence: Third-party packages can be compiled with different `rustc` versions, C, or C++ without ABI breakage.
- Eliminates undefined behavior caused by cross-allocator drops.
- Deterministic version handshake prevents loading incompatible or corrupted binaries.

### Negative / Trade-offs
- Writing low-level FFI bridges requires careful pointer handling compared to pure Rust trait objects.
- String serialization across the C ABI incurs minimal memcpy overhead for method calls (negligible for IPC/method calls).
