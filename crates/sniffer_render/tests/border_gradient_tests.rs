use sniffer_render::glow::batching::{QuadBatch, QuadInstanceData, unpack_color};

#[test]
fn test_quad_instance_gradient_border_data() {
    let top_color = 0xFFFF_0000; // Red
    let bottom_color = 0xFF00_00FF; // Blue
    let b_top = unpack_color(top_color);
    let b_bot = unpack_color(bottom_color);

    let inst = QuadInstanceData {
        rect_pos: [10.0, 20.0],
        rect_size: [100.0, 50.0],
        color: [0.1, 0.1, 0.1, 1.0],
        border_color: b_top,
        color_bottom: [0.1, 0.1, 0.1, 1.0],
        shape_size: [100.0, 50.0],
        is_circle: 0.0,
        is_shadow: 0.0,
        radius: 8.0,
        border_width: 2.0,
        shadow_blur: 0.0,
        is_gradient: 1.0,
        border_color_bottom: b_bot,
    };

    assert_eq!(inst.border_width, 2.0);
    assert_eq!(inst.border_color, b_top);
    assert_eq!(inst.border_color_bottom, b_bot);
    assert_eq!(inst.is_gradient, 1.0);
}

#[test]
fn test_batch_gradient_borders_alongside_solid_borders() {
    let mut batch = QuadBatch::new(64);

    // 10 cards with linear gradient borders
    for i in 0..10 {
        let inst = QuadInstanceData {
            rect_pos: [i as f32 * 30.0, 0.0],
            rect_size: [100.0, 50.0],
            color: [0.2, 0.2, 0.2, 1.0],
            border_color: [1.0, 0.0, 0.0, 1.0],
            color_bottom: [0.2, 0.2, 0.2, 1.0],
            shape_size: [100.0, 50.0],
            is_circle: 0.0,
            is_shadow: 0.0,
            radius: 8.0,
            border_width: 1.5,
            shadow_blur: 0.0,
            is_gradient: 1.0,
            border_color_bottom: [0.0, 0.0, 1.0, 1.0],
        };
        assert!(batch.push_instance(inst));
    }

    // 10 cards with solid borders
    for i in 10..20 {
        let inst = QuadInstanceData {
            rect_pos: [i as f32 * 30.0, 0.0],
            rect_size: [100.0, 50.0],
            color: [0.2, 0.2, 0.2, 1.0],
            border_color: [0.0, 1.0, 0.0, 1.0],
            color_bottom: [0.2, 0.2, 0.2, 1.0],
            shape_size: [100.0, 50.0],
            is_circle: 0.0,
            is_shadow: 0.0,
            radius: 8.0,
            border_width: 1.0,
            shadow_blur: 0.0,
            is_gradient: 0.0,
            border_color_bottom: [0.0, 1.0, 0.0, 1.0],
        };
        assert!(batch.push_instance(inst));
    }

    assert_eq!(
        batch.len(),
        20,
        "both gradient and solid border quads batch into unified buffer"
    );
}
