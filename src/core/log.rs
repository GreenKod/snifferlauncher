#[cfg(target_os = "android")]
#[allow(unused_extern_crates)]
unsafe extern "C" {
    fn __android_log_write(prio: i32, tag: *const std::ffi::c_char, text: *const std::ffi::c_char) -> i32;
}

#[cfg(target_os = "android")]
pub fn android_log(prio: i32, msg: &str) {
    if let (Ok(c_tag), Ok(c_msg)) = (
        std::ffi::CString::new("SnifferLauncher"),
        std::ffi::CString::new(msg),
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
        $crate::core::log::android_log(4, &msg);
        #[cfg(not(target_os = "android"))]
        println!("{msg}");
    }};
}

#[macro_export]
macro_rules! dev_err {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(target_os = "android")]
        $crate::core::log::android_log(6, &msg);
        #[cfg(not(target_os = "android"))]
        eprintln!("{msg}");
    }};
}
