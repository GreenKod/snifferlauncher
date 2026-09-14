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

#[cfg(target_os = "android")]
static ANDROID_WAKER: std::sync::OnceLock<android_activity::AndroidAppWaker> =
    std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_app(app: &android_activity::AndroidApp) {
    let _ = ANDROID_WAKER.set(app.create_waker());
    image_loader::set_android_app(app);
}

/// Reactively wakes up the main Android event loop if it is waiting in `poll_events`.
pub fn wake_app() {
    #[cfg(target_os = "android")]
    if let Some(waker) = ANDROID_WAKER.get() {
        waker.wake();
    }
}

#[cfg(not(target_os = "android"))]
pub fn launch_action(_action: sniffer_core::Action) -> Result<(), String> {
    Err("android launcher is only available on Android".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_app_does_not_panic() {
        // Calling wake_app when uninitialized must be a safe no-op.
        wake_app();
    }
}
