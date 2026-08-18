use sniffer_core::layout::LayoutNode;
use sniffer_core::math::{Point, Rect};
use sniffer_core::style::Style;
use sniffer_core::types::Element;
use sniffer_render::draw::culling::find_clicked_button;

#[test]
fn test_find_clicked_button() {
    let btn = Element::Container {
        id: Some("app_btn_1".to_string()),
        style: Style::default(),
        children: vec![],
    };
    let btn_node = LayoutNode {
        element: btn.clone(),
        rect: Rect::new(50.0, 100.0, 150.0, 60.0),
        children: vec![],
    };

    let root = Element::Container {
        id: Some("root".to_string()),
        style: Style::default(),
        children: vec![btn],
    };
    let root_node = LayoutNode {
        element: root.clone(),
        rect: Rect::new(0.0, 0.0, 800.0, 600.0),
        children: vec![btn_node],
    };

    // Hit inside button
    let hit = find_clicked_button(&root, &root_node, Point::new(100.0, 120.0));
    assert!(hit.is_some());
    let (id_hash, rect) = hit.unwrap();
    assert_eq!(id_hash, sniffer_core::ui::widget::fnv1a(b"app_btn_1"));
    assert_eq!(rect.x, 50.0);
    assert_eq!(rect.y, 100.0);

    // Miss button
    let miss = find_clicked_button(&root, &root_node, Point::new(300.0, 300.0));
    assert!(miss.is_some());
    let (miss_hash, _) = miss.unwrap();
    assert_eq!(miss_hash, sniffer_core::ui::widget::fnv1a(b"root"));
}
