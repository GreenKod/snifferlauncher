pub mod idle;
pub mod image_loader;

#[cfg(target_os = "android")]
pub mod app;
#[cfg(target_os = "android")]
pub mod input;
#[cfg(target_os = "android")]
pub mod jni;
#[cfg(target_os = "android")]
pub mod window;

pub use idle::IdleDetector;
pub use image_loader::{ImageLoadRequest, ImageLoadResult};

#[cfg(target_os = "android")]
pub use app::android_main;

#[cfg(target_os = "android")]
pub use jni::intent::launch_action;

#[cfg(not(target_os = "android"))]
pub fn launch_action(_action: sniffer_core::Action) -> Result<(), String> {
    Err("android launcher is only available on Android".to_string())
}
