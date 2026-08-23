//! Dynamic library loading for the package layer.

pub mod android;
pub mod dylib;

#[cfg(test)]
mod tests;

pub use android::prepare_android_dylib;
#[cfg(feature = "dynamic")]
pub use dylib::{
    DynamicHandle, load_service_dylib, load_service_dylib_with_symbol, load_widget_dylib,
    load_widget_dylib_with_symbol,
};
