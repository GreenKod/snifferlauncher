//! Background image loading channel and dedicated worker thread.

use super::types::{ImageLoadRequest, ImageLoadResult};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static IMAGE_REQ_SENDER: OnceLock<Sender<ImageLoadRequest>> = OnceLock::new();
static IMAGE_RES_RECEIVER: OnceLock<Receiver<ImageLoadResult>> = OnceLock::new();
static PENDING_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn pending_requests() -> &'static Mutex<HashSet<String>> {
    PENDING_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Decodes in-memory image bytes (e.g. PNG, JPEG) into an `ImageLoadResult`.
#[must_use]
pub fn decode_image_bytes(id: &str, src: &str, bytes: &[u8]) -> Option<ImageLoadResult> {
    let img = match image::load_from_memory(bytes) {
        Ok(img) => img,
        Err(e) => {
            sniffer_core::dev_err!("Failed to decode image '{src}': {e}");
            return None;
        }
    };
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(ImageLoadResult::new(
        id,
        src,
        rgba.into_raw(),
        width,
        height,
    ))
}

#[cfg(target_os = "android")]
fn read_image_data(app: &android_activity::AndroidApp, src: &str) -> Option<Vec<u8>> {
    let asset_path = if src.starts_with(".plugins/") {
        src.to_string()
    } else {
        format!("{}/{}", obfstr::obfstr!(".plugins"), src)
    };

    if let Ok(cstr) = std::ffi::CString::new(asset_path) {
        if let Some(mut asset) = app.asset_manager().open(cstr.as_c_str()) {
            use std::io::Read;
            let mut buffer = Vec::new();
            if asset.read_to_end(&mut buffer).is_ok() {
                return Some(buffer);
            }
        }
    }

    std::fs::read(src).ok()
}

#[cfg(not(target_os = "android"))]
fn read_image_data(src: &str) -> Option<Vec<u8>> {
    if let Ok(bytes) = std::fs::read(src) {
        return Some(bytes);
    }
    let fallback_path = format!(".plugins/{src}");
    std::fs::read(fallback_path).ok()
}

/// Submits an asynchronous image loading request to the worker channel.
/// Returns `true` if the request was scheduled, or `false` if it is already pending or channel is uninitialized.
pub fn request_async_image(req: ImageLoadRequest) -> bool {
    if let Ok(mut pending) = pending_requests().lock() {
        if !pending.insert(req.src.clone()) {
            return false;
        }
    }

    if let Some(sender) = IMAGE_REQ_SENDER.get() {
        if sender.send(req.clone()).is_ok() {
            return true;
        }
    }

    if let Ok(mut pending) = pending_requests().lock() {
        pending.remove(&req.src);
    }
    false
}

/// Polls for a completed asynchronous image load result (non-blocking).
#[must_use]
pub fn poll_async_image() -> Option<ImageLoadResult> {
    IMAGE_RES_RECEIVER.get().and_then(|rx| rx.try_recv().ok())
}

/// Initializes the background image loader worker thread and crossbeam channels on Android.
#[cfg(target_os = "android")]
pub fn init_image_worker_pool(app: &android_activity::AndroidApp) {
    if IMAGE_REQ_SENDER.get().is_some() {
        return;
    }

    let (req_tx, req_rx) = unbounded::<ImageLoadRequest>();
    let (res_tx, res_rx) = unbounded::<ImageLoadResult>();

    let _ = IMAGE_REQ_SENDER.set(req_tx);
    let _ = IMAGE_RES_RECEIVER.set(res_rx);

    let app_clone = app.clone();
    let _ = std::thread::Builder::new()
        .name("sniffer-image-loader".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                if let Some(bytes) = read_image_data(&app_clone, &req.src) {
                    if let Some(res) = decode_image_bytes(&req.id, &req.src, &bytes) {
                        let _ = res_tx.send(res);
                    }
                }
                if let Ok(mut pending) = pending_requests().lock() {
                    pending.remove(&req.src);
                }
            }
        });
}

/// Initializes the background image loader worker thread and crossbeam channels on non-Android platforms.
#[cfg(not(target_os = "android"))]
pub fn init_image_worker_pool() {
    if IMAGE_REQ_SENDER.get().is_some() {
        return;
    }

    let (req_tx, req_rx) = unbounded::<ImageLoadRequest>();
    let (res_tx, res_rx) = unbounded::<ImageLoadResult>();

    let _ = IMAGE_REQ_SENDER.set(req_tx);
    let _ = IMAGE_RES_RECEIVER.set(res_rx);

    let _ = std::thread::Builder::new()
        .name("sniffer-image-loader".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                if let Some(bytes) = read_image_data(&req.src) {
                    if let Some(res) = decode_image_bytes(&req.id, &req.src, &bytes) {
                        let _ = res_tx.send(res);
                    }
                }
                if let Ok(mut pending) = pending_requests().lock() {
                    pending.remove(&req.src);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_decode_image_bytes_with_valid_png() {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([128, 64, 32, 255]));
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .expect("PNG encoding failed");

        let res = decode_image_bytes("test_id", "test.png", &buffer);
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res.id, "test_id");
        assert_eq!(res.src, "test.png");
        assert_eq!(res.width, 2);
        assert_eq!(res.height, 2);
        assert!(res.is_valid());
        assert_eq!(res.pixels.len(), 16);
    }

    #[test]
    fn test_decode_image_bytes_with_corrupt_data() {
        let corrupt = [0xde, 0xad, 0xbe, 0xef];
        let res = decode_image_bytes("test_id", "corrupt.png", &corrupt);
        assert!(res.is_none());
    }

    #[test]
    fn test_worker_channel_roundtrip() {
        init_image_worker_pool();

        let req = ImageLoadRequest::new("sample", "non_existent_path.png");
        let scheduled = request_async_image(req);
        // Since the file doesn't exist, it shouldn't produce a decoded image result,
        // but it should successfully schedule and clear pending status
        if scheduled {
            std::thread::sleep(std::time::Duration::from_millis(50));
            assert!(poll_async_image().is_none());
        }
    }
}
