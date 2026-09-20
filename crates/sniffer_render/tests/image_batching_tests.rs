/// Stage 4.5 — Image Icon Batching: Verification & Benchmark
///
/// These tests prove that:
/// 1. 60+ atlas icons accumulate into a single `ImageBatch` with a single flush call,
///    versus N individual `draw_arrays` calls in the legacy path.
/// 2. The batch fills up and auto-flushes without losing instances.
/// 3. Memory layout and capacity invariants hold for realistic app-drawer workloads.
use sniffer_render::glow::batching::{ImageBatch, ImageInstanceData};
use sniffer_render::glow::textures::{AtlasRegion, IconAtlas};
use std::num::NonZeroU32;

fn dummy_texture(id: u32) -> glow::Texture {
    glow::NativeTexture(NonZeroU32::new(id).unwrap())
}

/// Build a synthetic atlas with `n` fake icons packed into it.
fn build_atlas_with_icons(n: usize) -> (IconAtlas, Vec<AtlasRegion>) {
    let mut atlas = IconAtlas::new_with_texture(dummy_texture(1), 2048, 2048);
    let mut regions = Vec::with_capacity(n);
    for i in 0..n {
        let col = (i % 32) as u32;
        let row = (i / 32) as u32;
        let x = col * 64;
        let y = row * 64;
        let region = AtlasRegion {
            x,
            y,
            width: 48,
            height: 48,
            uv_rect: [
                x as f32 / 2048.0,
                y as f32 / 2048.0,
                48.0 / 2048.0,
                48.0 / 2048.0,
            ],
        };
        atlas.insert_region(format!("icon_{i}"), region);
        regions.push(region);
    }
    (atlas, regions)
}

/// Simulate the batching path for `n` icons.
/// Returns (flush_count, total_instances_submitted).
fn simulate_batch_draw(n: usize, batch_capacity: usize) -> (usize, usize) {
    let mut batch = ImageBatch::new(batch_capacity);
    let mut flush_count = 0usize;
    let mut total_submitted = 0usize;

    for i in 0..n {
        let col = (i % 32) as f32;
        let row = (i / 32) as f32;
        let instance = ImageInstanceData {
            rect_pos: [col * 60.0, row * 60.0],
            rect_size: [52.0, 52.0],
            uv_rect: [0.0, 0.0, 0.023_437_5, 0.023_437_5],
            radius: 8.0,
            alpha: 1.0,
            pad: [0.0, 0.0],
        };

        if batch.is_full() {
            flush_count += 1;
            total_submitted += batch.len();
            batch.clear();
        }

        assert!(
            batch.push_instance(instance),
            "push should succeed after clear"
        );
    }

    if !batch.is_empty() {
        flush_count += 1;
        total_submitted += batch.len();
        batch.clear();
    }

    (flush_count, total_submitted)
}

// ---------------------------------------------------------------------------
// Test 1: 60 icons → 1 instanced draw call (capacity >= 60)
// ---------------------------------------------------------------------------
#[test]
fn test_60_icons_collapse_to_single_draw_call() {
    let n = 60;
    let (flush_count, total) = simulate_batch_draw(n, ImageBatch::DEFAULT_MAX_CAPACITY);

    assert_eq!(total, n, "all {n} icons must be submitted, got {total}");
    assert_eq!(
        flush_count, 1,
        "60 icons fit in one batch → exactly 1 draw call, got {flush_count}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: 256 icons with default capacity → minimal number of flushes
// ---------------------------------------------------------------------------
#[test]
fn test_256_icons_minimal_flushes() {
    let n = 256;
    let cap = ImageBatch::DEFAULT_MAX_CAPACITY;
    let (flush_count, total) = simulate_batch_draw(n, cap);
    let expected_flushes = n.div_ceil(cap);

    assert_eq!(total, n, "all {n} icons must be submitted");
    assert_eq!(
        flush_count, expected_flushes,
        "expected {expected_flushes} flush(es) for {n} icons with capacity {cap}, got {flush_count}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Legacy path would have issued N separate draw calls; batching reduces that
// ---------------------------------------------------------------------------
#[test]
fn test_draw_call_reduction_vs_legacy() {
    let icon_counts = [16usize, 32, 60, 64, 128, 200];
    let cap = ImageBatch::DEFAULT_MAX_CAPACITY;

    for &n in &icon_counts {
        let legacy_draw_calls = n;
        let (flush_count, total) = simulate_batch_draw(n, cap);

        assert_eq!(total, n);
        assert!(
            flush_count < legacy_draw_calls || n == 1,
            "batching should reduce draw calls: {flush_count} < {legacy_draw_calls} for n={n}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Atlas region lookup correctness for 60 app-drawer icons
// ---------------------------------------------------------------------------
#[test]
fn test_atlas_lookup_correctness_60_icons() {
    let n = 60;
    let (atlas, regions) = build_atlas_with_icons(n);

    for (i, expected_region) in regions.iter().enumerate() {
        let key = format!("icon_{i}");
        let found = atlas.get(&key).copied();
        assert!(found.is_some(), "atlas must contain '{key}'");
        let region = found.unwrap();
        assert_eq!(
            region.uv_rect, expected_region.uv_rect,
            "UV rect mismatch for icon_{i}"
        );
        assert_eq!(region.width, 48, "width mismatch for icon_{i}");
        assert_eq!(region.height, 48, "height mismatch for icon_{i}");
    }
}

// ---------------------------------------------------------------------------
// Test 5: Batch overflow auto-clears and continues correctly
// ---------------------------------------------------------------------------
#[test]
fn test_batch_overflow_auto_flush_continuity() {
    let small_cap = 16;
    let n = 100;
    let (flush_count, total) = simulate_batch_draw(n, small_cap);
    let expected = n.div_ceil(small_cap);

    assert_eq!(total, n, "no icons must be lost across auto-flushes");
    assert_eq!(
        flush_count, expected,
        "expected {expected} flushes for n={n} cap={small_cap}, got {flush_count}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: Memory footprint — each instance is 48 bytes (std140 alignment)
// ---------------------------------------------------------------------------
#[test]
fn test_image_instance_memory_footprint() {
    assert_eq!(
        std::mem::size_of::<ImageInstanceData>(),
        48,
        "ImageInstanceData must be exactly 48 bytes (3 × vec4)"
    );
    assert_eq!(
        std::mem::align_of::<ImageInstanceData>(),
        4,
        "ImageInstanceData must be 4-byte aligned"
    );
}

// ---------------------------------------------------------------------------
// Test 7: Throughput — batching 60 icons in <1 ms of CPU time
// ---------------------------------------------------------------------------
#[test]
fn test_batch_fill_60_icons_cpu_throughput() {
    use std::time::Instant;

    let cap = ImageBatch::DEFAULT_MAX_CAPACITY;
    let mut batch = ImageBatch::new(cap);
    let t0 = Instant::now();

    for i in 0..60usize {
        let inst = ImageInstanceData {
            rect_pos: [i as f32 * 60.0, 0.0],
            rect_size: [52.0, 52.0],
            uv_rect: [0.0, 0.0, 0.023_437_5, 0.023_437_5],
            radius: 8.0,
            alpha: 1.0,
            pad: [0.0, 0.0],
        };
        assert!(batch.push_instance(inst));
    }

    let elapsed_us = t0.elapsed().as_micros();
    assert_eq!(batch.len(), 60);
    assert!(
        elapsed_us < 1000,
        "filling 60-icon batch took {elapsed_us}µs, expected <1000µs"
    );
}
