use sniffer_core::math::Rect;
use sniffer_render::glow::batching::{QuadBatch, QuadInstanceData};

#[test]
fn test_dual_shadow_instance_construction_and_alpha_ratios() {
    let rect = Rect::new(100.0, 200.0, 300.0, 150.0);
    let radius = 16.0;
    let elevation = 8.0;
    let shadow_color = Some(0xFF00_0000); // 100% black
    let global_alpha = 1.0;

    let (instance, shadow_rect) =
        QuadInstanceData::new_dual_shadow(rect, radius, elevation, shadow_color, global_alpha);

    assert_eq!(
        instance.is_shadow, 2.0,
        "is_shadow must be 2.0 for dual shadow mode"
    );
    assert_eq!(instance.radius, radius);
    assert_eq!(instance.shape_size, [rect.width, rect.height]);

    // Key light (location 2: color) vs Ambient Occlusion (location 4: color_bottom)
    let key_alpha = instance.color[3];
    let ambient_alpha = instance.color_bottom[3];
    assert!(
        (key_alpha - 0.65).abs() < 1e-4,
        "key light alpha should be ~0.65, got {key_alpha}"
    );
    assert!(
        (ambient_alpha - 0.35).abs() < 1e-4,
        "ambient light alpha should be ~0.35, got {ambient_alpha}"
    );

    // Bounding rect must enclose the base rect symmetrically so centers coincide
    let rect_cx = rect.x + rect.width * 0.5;
    let rect_cy = rect.y + rect.height * 0.5;
    let shadow_cx = shadow_rect.x + shadow_rect.width * 0.5;
    let shadow_cy = shadow_rect.y + shadow_rect.height * 0.5;
    assert!(
        (rect_cx - shadow_cx).abs() < 1e-4,
        "shadow center X must match rect center X"
    );
    assert!(
        (rect_cy - shadow_cy).abs() < 1e-4,
        "shadow center Y must match rect center Y"
    );

    assert!(
        shadow_rect.width > rect.width,
        "shadow quad must be larger than rect"
    );
    assert!(
        shadow_rect.height > rect.height,
        "shadow quad must be larger than rect"
    );
}

#[test]
fn test_dual_shadow_default_color_fallback() {
    let rect = Rect::new(50.0, 50.0, 200.0, 100.0);
    let (instance, _) = QuadInstanceData::new_dual_shadow(rect, 8.0, 4.0, None, 1.0);

    // Default color is 0x4D00_0000 (~0.3 alpha)
    let base_alpha = 0x4D as f32 / 255.0;
    assert!((instance.color[3] - base_alpha * 0.65).abs() < 1e-3);
    assert!((instance.color_bottom[3] - base_alpha * 0.35).abs() < 1e-3);
}

#[test]
fn test_batching_multiple_shadows_and_cards() {
    let mut batch = QuadBatch::new(128);

    for i in 0..20 {
        let rect = Rect::new(i as f32 * 20.0, 100.0, 150.0, 80.0);
        let (shadow_inst, _) = QuadInstanceData::new_dual_shadow(rect, 12.0, 6.0, None, 1.0);
        assert!(
            batch.push_instance(shadow_inst),
            "dual shadow instance push must succeed"
        );

        let card_inst = QuadInstanceData {
            rect_pos: [rect.x, rect.y],
            rect_size: [rect.width, rect.height],
            color: [0.1, 0.1, 0.2, 1.0],
            border_color: [0.0; 4],
            color_bottom: [0.1, 0.1, 0.2, 1.0],
            shape_size: [rect.width, rect.height],
            is_circle: 0.0,
            is_shadow: 0.0,
            radius: 12.0,
            border_width: 0.0,
            shadow_blur: 0.0,
            is_gradient: 0.0,
            border_color_bottom: [0.0; 4],
        };
        assert!(
            batch.push_instance(card_inst),
            "card instance push must succeed"
        );
    }

    // 20 shadows + 20 cards = 40 instances in ONE unified batch
    assert_eq!(batch.len(), 40);
    assert!(!batch.is_full());
}

#[test]
fn test_shadow_overdraw_bounding_and_clamping() {
    let rect = Rect::new(100.0, 100.0, 200.0, 100.0);

    // 1. Reasonable elevation (8.0): padding should be bounded to reach (~21.4px)
    let (_, shadow_rect_normal) = QuadInstanceData::new_dual_shadow(rect, 8.0, 8.0, None, 1.0);
    let pad_normal = (shadow_rect_normal.width - rect.width) * 0.5;
    assert!(
        pad_normal < 30.0,
        "padding should be tightly bounded to SDF reach, got {pad_normal}"
    );

    // 2. Extreme elevation (120.0): without bounding this would add > 400px of overdraw!
    // With MAX_SHADOW_PADDING (64.0), padding must be clamped at exactly 64.0px.
    let (_, shadow_rect_extreme) = QuadInstanceData::new_dual_shadow(rect, 8.0, 120.0, None, 1.0);
    let pad_extreme = (shadow_rect_extreme.width - rect.width) * 0.5;
    assert!(
        (pad_extreme - 64.0).abs() < 1e-4,
        "extreme elevation shadow padding must be clamped to MAX_SHADOW_PADDING (64.0px), got {pad_extreme}"
    );

    assert_eq!(shadow_rect_extreme.width, rect.width + 128.0);
    assert_eq!(shadow_rect_extreme.height, rect.height + 128.0);
}
