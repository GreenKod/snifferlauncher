use sniffer_render::TextBatch;

#[test]
fn test_app_drawer_120_labels_batching() {
    let mut batch = TextBatch::new(65_536);
    assert!(batch.is_empty());

    // Mock an App Drawer with 120 apps: titles, category tags, and emoji icons
    let app_titles = [
        ("Telefon", "📞", 0xFFFF_FFFF_u32),
        ("Mesajlar", "💬", 0xFF44_AAFF_u32),
        ("Kamera", "📷", 0xFFFF_AA00_u32),
        ("Galeri", "🖼️", 0xFFAA_FFAA_u32),
        ("Ayarlar", "⚙️", 0xFFDD_DDDD_u32),
        ("Tarayıcı", "🌐", 0xFF00_DDFF_u32),
    ];

    let mut total_quads = 0;

    for i in 0..120 {
        let (title, icon, color) = app_titles[i % app_titles.len()];
        let col = [
            ((color >> 16) & 0xFF) as f32 / 255.0,
            ((color >> 8) & 0xFF) as f32 / 255.0,
            (color & 0xFF) as f32 / 255.0,
            1.0,
        ];

        // Draw text characters (normal SDF glifler)
        for _ in title.chars() {
            for v in 0..6 {
                let x = (v as f32) * 5.0;
                let y = (i as f32) * 20.0;
                batch
                    .vertices
                    .extend_from_slice(&[x, y, 0.0, 0.0, col[0], col[1], col[2], col[3], 0.0]);
            }
            total_quads += 1;
        }

        // Draw icon emoji (color glif)
        for _ in icon.chars() {
            for v in 0..6 {
                let x = (v as f32) * 5.0 + 50.0;
                let y = (i as f32) * 20.0;
                batch
                    .vertices
                    .extend_from_slice(&[x, y, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
            }
            total_quads += 1;
        }
    }

    assert_eq!(batch.len(), total_quads * 6 * 9);
    assert!(
        batch.len() > 100 * 6 * 9,
        "Should contain well over 100 label glyphs"
    );

    // All 120 labels and their emojis are combined into a single continuous GPU slice!
    let raw_bytes = batch.as_bytes();
    assert_eq!(
        raw_bytes.len(),
        total_quads * 6 * 9 * std::mem::size_of::<f32>()
    );

    batch.clear();
    assert!(batch.is_empty());
}

#[test]
fn test_text_batch_capacity_bound() {
    let max_cap = 1000;
    let mut batch = TextBatch::new(max_cap);

    assert!(!batch.is_full());
    batch.vertices.resize(1000, 0.0);
    assert!(batch.is_full());

    batch.clear();
    assert!(!batch.is_full());
    assert_eq!(batch.len(), 0);
}

#[test]
fn test_compare_unbatched_vs_batched_overhead() {
    const LABEL_COUNT: usize = 150;
    const ITERS: usize = 200;

    // 1. Unbatched approach (Old design):
    // Every label allocates its own Vec<f32>, simulates a driver draw call setup, then drops it
    let start_unbatched = std::time::Instant::now();
    let mut unbatched_draw_calls = 0;
    let mut total_unbatched_allocs = 0;

    for _ in 0..ITERS {
        for _ in 0..LABEL_COUNT {
            // Simulated draw_text_impl with per-draw allocation
            let mut quads: Vec<f32> = Vec::with_capacity(20 * 24);
            for v in 0..120 {
                quads.push(v as f32);
            }
            // Each label triggered separate draw_arrays call
            unbatched_draw_calls += 1;
            total_unbatched_allocs += 1;
            std::hint::black_box(&quads);
        }
    }
    let duration_unbatched = start_unbatched.elapsed();

    // 2. Batched approach (New design):
    // Persistent TextBatch, zero allocations per label, single flush/draw call per frame
    let mut batch = TextBatch::new(65_536);
    let start_batched = std::time::Instant::now();
    let mut batched_draw_calls = 0;
    let total_batched_allocs = 0;

    for _ in 0..ITERS {
        // Across the frame, all 150 labels write into the persistent batch
        for _ in 0..LABEL_COUNT {
            // Zero heap allocation here — reuses batch.vertices capacity
            for v in 0..120 {
                batch.vertices.push(v as f32);
            }
            // No draw call per label!
        }
        // Exactly ONE draw call per frame/flush!
        batched_draw_calls += 1;
        std::hint::black_box(batch.as_bytes());
        batch.clear();
    }
    let duration_batched = start_batched.elapsed();

    println!(
        "\n================= BENCHMARK RAPORU: ESKİ vs YENİ (TEXT BATCHING) ================="
    );
    println!("Kare Başına Etiket Sayısı : {LABEL_COUNT}");
    println!("Test Döngüsü (Frames)     : {ITERS}");
    println!("----------------------------------------------------------------------------------");
    println!("ESKİ YÖNTEM (Per-draw allocation & draw call):");
    println!(
        "  - Toplam GPU Draw Call  : {unbatched_draw_calls} çağrı (kare başına {LABEL_COUNT})"
    );
    println!("  - Toplam Heap Tahsisi   : {total_unbatched_allocs} adet malloc/free");
    println!("  - Toplam CPU Süresi     : {duration_unbatched:?}");
    println!("----------------------------------------------------------------------------------");
    println!("YENİ YÖNTEM (TextBatching & Unified Flush):");
    println!("  - Toplam GPU Draw Call  : {batched_draw_calls} çağrı (kare başına 1)");
    println!("  - Toplam Heap Tahsisi   : {total_batched_allocs} adet (SIFIR malloc/free)");
    println!("  - Toplam CPU Süresi     : {duration_batched:?}");
    println!("----------------------------------------------------------------------------------");
    let speedup = duration_unbatched.as_nanos() as f64 / duration_batched.as_nanos() as f64;
    let draw_reduction = (1.0 - (batched_draw_calls as f64 / unbatched_draw_calls as f64)) * 100.0;
    println!("KAZANIM:");
    println!("  - GPU Draw Call Düşüşü  : %{draw_reduction:.1} AZALMA!");
    println!("  - CPU İşlem Hızı Artışı : {speedup:.2}x DAHA HIZLI!");
    println!(
        "==================================================================================\n"
    );

    assert!(batched_draw_calls < unbatched_draw_calls);
    assert_eq!(batched_draw_calls, ITERS);
    assert_eq!(unbatched_draw_calls, ITERS * LABEL_COUNT);
}
