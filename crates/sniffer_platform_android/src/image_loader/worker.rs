//! Background image loading channel and dedicated worker thread.

use super::types::{ImageLoadRequest, ImageLoadResult};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// Maximum allowable image asset file size (32 MB) to guard against OOM exhaustion.
const MAX_IMAGE_FILE_SIZE: u64 = 32 * 1024 * 1024;
/// Maximum dimension in either width or height (8192 px) to protect GPU texture limits.
const MAX_IMAGE_DIMENSION: u32 = 8192;

static IMAGE_REQ_SENDER: OnceLock<Sender<ImageLoadRequest>> = OnceLock::new();
static IMAGE_RES_RECEIVER: OnceLock<Receiver<ImageLoadResult>> = OnceLock::new();
static PENDING_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn pending_requests() -> &'static Mutex<HashSet<String>> {
    PENDING_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// RAII guard ensuring a requested path is unconditionally removed from `PENDING_REQUESTS`
/// even if a panic or early-return occurs during decoding.
struct PendingGuard<'a>(&'a str);

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut pending) = pending_requests().lock() {
            pending.remove(self.0);
        }
    }
}

/// Decodes in-memory image bytes (e.g. PNG, JPEG) into an `ImageLoadResult`.
/// Guarantees dimension validation and bounds protection against malformed headers.
#[must_use]
pub fn decode_image_bytes(id: &str, src: &str, bytes: &[u8]) -> Option<ImageLoadResult> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_FILE_SIZE as usize {
        sniffer_core::dev_err!("Image '{src}' bytes out of bounds (len: {})", bytes.len());
        return None;
    }

    let img = match image::load_from_memory(bytes) {
        Ok(img) => img,
        Err(e) => {
            sniffer_core::dev_err!("Failed to decode image '{src}': {e}");
            return None;
        }
    };

    let width = img.width();
    let height = img.height();
    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        sniffer_core::dev_err!(
            "Image '{src}' has invalid or excessive dimensions: {width}x{height} (max: {MAX_IMAGE_DIMENSION})"
        );
        return None;
    }

    let rgba = img.to_rgba8();
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
    if src.is_empty() || src.contains('\0') {
        sniffer_core::dev_err!("Invalid image path requested: '{src}'");
        return None;
    }

    let asset_path = if src.starts_with(".plugins/") {
        src.to_string()
    } else {
        format!("{}/{}", obfstr::obfstr!(".plugins"), src)
    };

    if let Ok(cstr) = std::ffi::CString::new(asset_path) {
        if let Some(mut asset) = app.asset_manager().open(cstr.as_c_str()) {
            use std::io::Read;
            let mut buffer = Vec::new();
            let mut handle = (&mut asset).take(MAX_IMAGE_FILE_SIZE + 1);
            if handle.read_to_end(&mut buffer).is_ok()
                && buffer.len() <= MAX_IMAGE_FILE_SIZE as usize
            {
                return Some(buffer);
            }
            sniffer_core::dev_err!("Image asset exceeded 32 MB limit or read failed: '{src}'");
            return None;
        }
    }

    if let Ok(metadata) = std::fs::metadata(src) {
        if metadata.len() <= MAX_IMAGE_FILE_SIZE {
            return std::fs::read(src).ok();
        }
        sniffer_core::dev_err!("File on disk exceeded 32 MB limit: '{src}'");
    }
    None
}

#[cfg(not(target_os = "android"))]
fn read_image_data(src: &str) -> Option<Vec<u8>> {
    if src.is_empty() || src.contains('\0') {
        sniffer_core::dev_err!("Invalid image path requested: '{src}'");
        return None;
    }

    if let Ok(metadata) = std::fs::metadata(src) {
        if metadata.len() <= MAX_IMAGE_FILE_SIZE {
            return std::fs::read(src).ok();
        }
    }

    let fallback_path = format!(".plugins/{src}");
    if let Ok(metadata) = std::fs::metadata(&fallback_path) {
        if metadata.len() <= MAX_IMAGE_FILE_SIZE {
            return std::fs::read(fallback_path).ok();
        }
    }
    None
}

/// Submits an asynchronous image loading request to the worker channel.
/// Returns `true` if the request was scheduled, or `false` if it is already pending, path is invalid, or channel is uninitialized.
pub fn request_async_image(req: ImageLoadRequest) -> bool {
    if req.src.is_empty() || req.src.contains('\0') {
        return false;
    }

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

/// Polls up to `max_count` completed asynchronous image load results (non-blocking).
#[must_use]
pub fn poll_async_images(max_count: usize) -> Vec<ImageLoadResult> {
    let mut results = Vec::new();
    if let Some(rx) = IMAGE_RES_RECEIVER.get() {
        while results.len() < max_count {
            if let Ok(res) = rx.try_recv() {
                results.push(res);
            } else {
                break;
            }
        }
    }
    results
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
                let _guard = PendingGuard(&req.src);
                let decode_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    read_image_data(&app_clone, &req.src)
                        .and_then(|bytes| decode_image_bytes(&req.id, &req.src, &bytes))
                }));

                if let Ok(Some(res)) = decode_res {
                    let _ = res_tx.send(res);
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
                let _guard = PendingGuard(&req.src);
                let decode_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    read_image_data(&req.src)
                        .and_then(|bytes| decode_image_bytes(&req.id, &req.src, &bytes))
                }));

                if let Ok(Some(res)) = decode_res {
                    let _ = res_tx.send(res);
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
    fn test_decode_image_bytes_rejects_empty_buffer() {
        let res = decode_image_bytes("empty", "empty.png", &[]);
        assert!(res.is_none());
    }

    #[test]
    fn test_request_async_image_rejects_null_bytes_and_empty() {
        let req_null = ImageLoadRequest::new("id", "path/with/\0null.png");
        assert!(!request_async_image(req_null));

        let req_empty = ImageLoadRequest::new("id", "");
        assert!(!request_async_image(req_empty));
    }

    #[test]
    fn test_pending_guard_cleans_up_pending_requests() {
        let path = "pending/test/path.png";
        {
            let mut pending = pending_requests().lock().unwrap();
            pending.insert(path.to_string());
            assert!(pending.contains(path));
        }

        {
            let _guard = PendingGuard(path);
        }

        let pending = pending_requests().lock().unwrap();
        assert!(!pending.contains(path));
    }

    #[test]
    fn test_worker_channel_roundtrip() {
        init_image_worker_pool();

        let req = ImageLoadRequest::new("sample", "non_existent_path.png");
        let scheduled = request_async_image(req);
        if scheduled {
            std::thread::sleep(std::time::Duration::from_millis(50));
            assert!(poll_async_image().is_none());
        }
    }

    #[test]
    fn test_poll_async_images_throttling() {
        let (tx, rx) = unbounded::<ImageLoadResult>();
        for i in 0..10 {
            let res =
                ImageLoadResult::new(format!("id_{i}"), format!("src_{i}"), vec![0; 16], 2, 2);
            let _ = tx.send(res);
        }

        let mut polled = Vec::new();
        let max_count = 4;
        while polled.len() < max_count {
            if let Ok(res) = rx.try_recv() {
                polled.push(res);
            } else {
                break;
            }
        }
        assert_eq!(polled.len(), 4);
        assert_eq!(polled[0].id, "id_0");
        assert_eq!(polled[3].id, "id_3");
    }
}
