use sniffer_core::math::Rect;
use sniffer_render::draw::culling::is_aabb_visible;

#[test]
fn test_aabb_culling_inside_and_outside() {
    let viewport = Rect::new(0.0, 0.0, 720.0, 1612.0);

    // Fully inside
    let r_inside = Rect::new(100.0, 100.0, 200.0, 200.0);
    assert!(is_aabb_visible(r_inside, viewport, 20.0, 20.0));

    // Outside to the right
    let r_right = Rect::new(1000.0, 100.0, 200.0, 200.0);
    assert!(!is_aabb_visible(r_right, viewport, 20.0, 20.0));

    // Outside to the bottom
    let r_bottom = Rect::new(100.0, 2000.0, 200.0, 200.0);
    assert!(!is_aabb_visible(r_bottom, viewport, 20.0, 20.0));

    // Outside to the left
    let r_left = Rect::new(-300.0, 100.0, 200.0, 200.0);
    assert!(!is_aabb_visible(r_left, viewport, 20.0, 20.0));

    // Crossing boundary with margin
    let r_edge = Rect::new(730.0, 100.0, 50.0, 50.0);
    assert!(is_aabb_visible(r_edge, viewport, 50.0, 50.0));
    assert!(!is_aabb_visible(r_edge, viewport, 5.0, 5.0));
}
