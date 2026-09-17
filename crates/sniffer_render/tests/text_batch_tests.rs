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
