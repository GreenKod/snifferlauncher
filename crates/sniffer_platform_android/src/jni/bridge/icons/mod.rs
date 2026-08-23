pub mod decoder;
pub mod worker;

pub(crate) use decoder::extract_drawable_pixels;
pub use worker::{
    IconLoadRequest, IconLoadResult, get_app_icon_pixels, init_icon_worker_pool,
    poll_async_app_icon, prefetch_app_icons, request_async_app_icon,
};
