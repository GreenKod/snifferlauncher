// Re-export SnifferEngine facade modules for the launcher host and external consumers
pub use sniffer_engine::{core, pkg, platform, plugin, prelude, render};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "Rust" fn android_main(app: android_activity::AndroidApp) {
    sniffer_engine::run_android(app);
}
