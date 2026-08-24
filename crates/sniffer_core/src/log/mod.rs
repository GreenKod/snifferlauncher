/// Cross-platform logging backend for `sniffer_core`.
///
/// On Android the macros delegate to `__android_log_write` via a minimal inline
/// FFI binding.  The binding lives here rather than in `sniffer_platform_android`
/// because `sniffer_core` is a dependency *of* the platform crate — inverting that
/// relationship would create a dependency cycle.  Using `#[cfg(target_os)]` keeps
/// the Android-specific code fully inert on every other target.

#[cfg(target_os = "android")]
#[allow(unused_extern_crates)]
unsafe extern "C" {
    fn __android_log_write(
        prio: i32,
        tag: *const std::ffi::c_char,
        text: *const std::ffi::c_char,
    ) -> i32;
}

/// Write `msg` to Android logcat at the given priority (4 = INFO, 6 = ERROR).
#[cfg(target_os = "android")]
pub fn android_log(prio: i32, msg: &str) {
    let clean_msg = msg.replace('\0', "\\0");
    if let (Ok(c_tag), Ok(c_msg)) = (
        std::ffi::CString::new("SnifferLauncher"),
        std::ffi::CString::new(clean_msg),
    ) {
        unsafe {
            __android_log_write(prio, c_tag.as_ptr(), c_msg.as_ptr());
        }
    }
}

#[macro_export]
macro_rules! dev_log {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(target_os = "android")]
        $crate::log::android_log(4, &msg);
        #[cfg(not(target_os = "android"))]
        println!("{msg}");
    }};
}

#[macro_export]
macro_rules! dev_err {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(target_os = "android")]
        $crate::log::android_log(6, &msg);
        #[cfg(not(target_os = "android"))]
        eprintln!("{msg}");
    }};
}
