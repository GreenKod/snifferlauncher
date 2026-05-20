pub mod android;
pub mod core;
#[cfg(not(target_os = "android"))]
pub mod desktop;
