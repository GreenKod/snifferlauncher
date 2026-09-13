//! Asynchronous image loading module for `sniffer_platform_android`.

pub mod types;
pub mod worker;

pub use types::{ImageLoadRequest, ImageLoadResult};
pub use worker::{
    decode_image_bytes, init_image_worker_pool, poll_async_image, request_async_image,
};
