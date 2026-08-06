/// Developer Kit logging macros controlled by the `devkit` Cargo feature.
///
/// When `devkit` feature is NOT enabled (standard release production build):
/// - `if cfg!(feature = "devkit")` evaluates to `if false { ... }`.
/// - Dead-code elimination (DCE) removes the call, text strings, and formatting code.
/// - 0 runtime overhead, 0 plaintext log/error strings in binary.
///
/// When `devkit` feature IS enabled (`cargo build --features devkit`):
/// - `if cfg!(feature = "devkit")` evaluates to `if true { ... }`.
/// - Full debug logging is enabled for plugin developers to diagnose IPC calls, permissions, and manifest issues.

#[macro_export]
macro_rules! dev_log {
    ($($arg:tt)*) => {
        if cfg!(feature = "devkit") {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! dev_err {
    ($($arg:tt)*) => {
        if cfg!(feature = "devkit") {
            eprintln!($($arg)*);
        }
    };
}
