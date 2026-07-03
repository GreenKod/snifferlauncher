#[cfg(target_os = "android")]
pub mod app;
#[cfg(target_os = "android")]
pub mod input;
#[cfg(target_os = "android")]
pub mod jni;
#[cfg(target_os = "android")]
pub mod types;
#[cfg(target_os = "android")]
pub mod window;

#[cfg(target_os = "android")]
pub use app::android_main;

#[cfg(target_os = "android")]
pub use jni::intent::launch_action;

#[cfg(not(target_os = "android"))]
/// Launch the requested action on non-Android targets.
///
/// # Errors
///
/// Always returns an error because action launching is only implemented on Android.
pub fn launch_action(_action: crate::core::Action) -> Result<(), String> {
    Err("android launcher is only available on Android".to_string())
}
