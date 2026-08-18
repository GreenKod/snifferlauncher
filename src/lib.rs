// Re-export workspace crates for consumers and backward-compatibility
pub use sniffer_core as core;
pub use sniffer_plugin as plugin;
pub use sniffer_render as render;

#[cfg(target_os = "android")]
pub use sniffer_platform_android as platform;

#[cfg(not(target_os = "android"))]
pub use sniffer_platform_desktop as platform;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: android_activity::AndroidApp) {
    sniffer_platform_android::android_main(app);
}
