//! Asynchronous image loading module for `sniffer_platform_android`.

pub mod types;
pub mod worker;

pub use types::{ImageLoadRequest, ImageLoadResult};
#[cfg(target_os = "android")]
pub use worker::set_android_app;
pub use worker::{
    decode_image_bytes, init_image_worker_pool, poll_async_image, poll_async_images,
    request_async_image,
};
