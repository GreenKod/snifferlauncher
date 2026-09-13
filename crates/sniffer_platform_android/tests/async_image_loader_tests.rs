use image::{ImageFormat, Rgba, RgbaImage};
use sniffer_platform_android::image_loader::{
    ImageLoadRequest, decode_image_bytes, init_image_worker_pool, poll_async_images,
    request_async_image,
};
use std::fs;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn create_sample_png_bytes(w: u32, h: u32, color: [u8; 4]) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, Rgba(color));
    let mut buffer = Vec::new();
    img.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
        .expect("PNG encoding failed");
    buffer
}

#[test]
fn test_async_image_loader_e2e_and_non_blocking_performance() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_image_worker_pool();

    let temp_dir = std::env::temp_dir().join("sniffer_test_img_loader");
    let _ = fs::create_dir_all(&temp_dir);
    let sample_file = temp_dir.join("test_card.png");
    let png_bytes = create_sample_png_bytes(64, 64, [255, 0, 128, 255]);
    fs::write(&sample_file, &png_bytes).expect("Failed to write test image");

    let file_str = sample_file.to_str().unwrap().to_string();

    // 1. Measure dispatch latency (simulating render thread caller)
    let req = ImageLoadRequest::new("card_1", &file_str);
    let start_dispatch = Instant::now();
    let scheduled = request_async_image(req);
    let dispatch_latency = start_dispatch.elapsed();

    assert!(scheduled, "Initial image request should be scheduled");
    assert!(
        dispatch_latency < Duration::from_millis(5),
        "Dispatch took too long: {dispatch_latency:?}"
    );

    // 2. Poll for the decoded result from worker thread
    let start_wait = Instant::now();
    let mut decoded = None;
    while start_wait.elapsed() < Duration::from_secs(2) {
        let results = poll_async_images(4);
        if let Some(res) = results.into_iter().find(|r| r.src == file_str) {
            decoded = Some(res);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(
        decoded.is_some(),
        "Async image loader failed to produce decoded result in time"
    );
    let res = decoded.unwrap();
    assert_eq!(res.id, "card_1");
    assert_eq!(res.src, file_str);
    assert_eq!(res.width, 64);
    assert_eq!(res.height, 64);
    assert!(res.is_valid());
    assert_eq!(res.pixels.len(), 64 * 64 * 4);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_async_image_loader_deduplication() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_image_worker_pool();

    let temp_dir = std::env::temp_dir().join("sniffer_test_img_dedup");
    let _ = fs::create_dir_all(&temp_dir);
    let sample_file = temp_dir.join("dedup_icon.png");
    let png_bytes = create_sample_png_bytes(32, 32, [0, 200, 100, 255]);
    fs::write(&sample_file, &png_bytes).expect("Failed to write test image");
    let file_str = sample_file.to_str().unwrap().to_string();

    let req1 = ImageLoadRequest::new("icon", &file_str);
    let req2 = ImageLoadRequest::new("icon", &file_str);
    let req3 = ImageLoadRequest::new("icon", &file_str);

    let first = request_async_image(req1);
    let second = request_async_image(req2);
    let third = request_async_image(req3);

    assert!(first, "First request must be accepted");
    assert!(!second, "Duplicate request while pending must be rejected");
    assert!(!third, "Duplicate request while pending must be rejected");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_async_image_loader_corruption_and_resilience() {
    let _lock = TEST_MUTEX.lock().unwrap();
    init_image_worker_pool();

    let temp_dir = std::env::temp_dir().join("sniffer_test_img_resilience");
    let _ = fs::create_dir_all(&temp_dir);

    // 1. Corrupt file
    let corrupt_file = temp_dir.join("corrupt.png");
    fs::write(&corrupt_file, b"NOT_A_REAL_PNG_HEADER_CORRUPT").expect("write corrupt file");
    let corrupt_str = corrupt_file.to_str().unwrap().to_string();

    // 2. Empty file
    let empty_file = temp_dir.join("empty.png");
    fs::write(&empty_file, b"").expect("write empty file");
    let empty_str = empty_file.to_str().unwrap().to_string();

    // 3. Valid file
    let valid_file = temp_dir.join("valid.png");
    let png_bytes = create_sample_png_bytes(16, 16, [10, 20, 30, 255]);
    fs::write(&valid_file, &png_bytes).expect("write valid file");
    let valid_str = valid_file.to_str().unwrap().to_string();

    // Submit invalid/corrupt requests
    assert!(request_async_image(ImageLoadRequest::new(
        "corrupt",
        &corrupt_str
    )));
    assert!(request_async_image(ImageLoadRequest::new(
        "empty", &empty_str
    )));
    // Submit invalid path with null byte
    assert!(!request_async_image(ImageLoadRequest::new(
        "null",
        "bad/\0path.png"
    )));

    // Submit valid request afterwards to prove worker remains alive and healthy
    assert!(request_async_image(ImageLoadRequest::new(
        "valid", &valid_str
    )));

    let start = Instant::now();
    let mut valid_received = false;
    while start.elapsed() < Duration::from_secs(2) {
        for res in poll_async_images(8) {
            if res.src == valid_str {
                valid_received = true;
                assert_eq!(res.width, 16);
                assert_eq!(res.height, 16);
                assert!(res.is_valid());
            }
            // Corrupt or empty requests should never yield valid decoded results
            assert_ne!(res.src, corrupt_str);
            assert_ne!(res.src, empty_str);
        }
        if valid_received {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(
        valid_received,
        "Worker failed to decode valid image after handling corrupt files"
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_sync_vs_async_decode_performance_benchmark() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let png_bytes = create_sample_png_bytes(256, 256, [200, 150, 100, 255]);

    // Benchmark synchronous decode (old render_thread approach)
    let sync_start = Instant::now();
    let sync_res = decode_image_bytes("sync_bench", "bench.png", &png_bytes);
    let sync_duration = sync_start.elapsed();

    assert!(sync_res.is_some());
    let sync_img = sync_res.unwrap();
    assert_eq!(sync_img.width, 256);
    assert_eq!(sync_img.height, 256);

    // Benchmark async dispatch latency (new render_thread approach)
    init_image_worker_pool();
    let req = ImageLoadRequest::new("async_bench", "virtual/test/bench.png");
    let async_dispatch_start = Instant::now();
    let scheduled = request_async_image(req);
    let async_dispatch_duration = async_dispatch_start.elapsed();

    assert!(scheduled);

    // Output performance metrics for analysis
    println!(
        "\n[Benchmark Analysis] 256x256 RGBA Image Decode:\n  Synchronous Decode (Old Render Loop): {:?}\n  Async Channel Dispatch (New Render Loop): {:?}\n  Latency Reduction Factor: {:.1}x",
        sync_duration,
        async_dispatch_duration,
        sync_duration.as_nanos() as f64 / (async_dispatch_duration.as_nanos().max(1) as f64)
    );
}
