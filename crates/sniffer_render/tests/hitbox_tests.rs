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

    // Hit outside button on empty space of root container -> must return None (not root!)
    let hit_outside = find_clicked_button(&root, &root_node, Point::new(10.0, 10.0));
    assert_eq!(hit_outside, None);
}

#[test]
fn test_invisible_layer_and_structural_wrapper_hitbox() {
    // A button inside drawer_grid_wrapper inside app_drawer_root
    let drawer_card = Element::Container {
        id: Some("drawer_card_0".to_string()),
        style: Style::default(),
        children: vec![],
    };
    let card_node = LayoutNode {
        element: drawer_card.clone(),
        rect: Rect::new(20.0, 50.0, 100.0, 100.0),
        children: vec![],
    };

    let grid_wrapper = Element::Container {
        id: Some("drawer_grid_wrapper".to_string()),
        style: Style::default(),
        children: vec![drawer_card],
    };
    let grid_node = LayoutNode {
        element: grid_wrapper.clone(),
        rect: Rect::new(0.0, 40.0, 400.0, 800.0),
        children: vec![card_node],
    };

    // Case 1: Drawer is invisible (opacity: 0.0)
    let hidden_style = Style {
        opacity: 0.0,
        ..Style::default()
    };
    let hidden_drawer_root = Element::Container {
        id: Some("app_drawer_root".to_string()),
        style: hidden_style,
        children: vec![grid_wrapper.clone()],
    };
    let hidden_root_node = LayoutNode {
        element: hidden_drawer_root.clone(),
        rect: Rect::new(0.0, 0.0, 400.0, 900.0),
        children: vec![grid_node.clone()],
    };

    // Touching inside drawer card while drawer is hidden -> MUST return None!
    let hit_hidden = find_clicked_button(
        &hidden_drawer_root,
        &hidden_root_node,
        Point::new(50.0, 80.0),
    );
    assert_eq!(hit_hidden, None);

    // Touching empty space of hidden grid_wrapper -> MUST return None!
    let hit_hidden_empty = find_clicked_button(
        &hidden_drawer_root,
        &hidden_root_node,
        Point::new(300.0, 500.0),
    );
    assert_eq!(hit_hidden_empty, None);

    // Case 2: Drawer is visible (opacity: 1.0)
    let visible_drawer_root = Element::Container {
        id: Some("app_drawer_root".to_string()),
        style: Style::default(),
        children: vec![grid_wrapper],
    };
    let visible_root_node = LayoutNode {
        element: visible_drawer_root.clone(),
        rect: Rect::new(0.0, 0.0, 400.0, 900.0),
        children: vec![grid_node],
    };

    // Touching drawer card -> hits drawer card!
    let hit_visible_card = find_clicked_button(
        &visible_drawer_root,
        &visible_root_node,
        Point::new(50.0, 80.0),
    );
    assert!(hit_visible_card.is_some());
    assert_eq!(
        hit_visible_card.unwrap().0,
        sniffer_core::ui::widget::fnv1a(b"drawer_card_0")
    );

    // Touching empty space of drawer_grid_wrapper -> MUST return None (not drawer_grid_wrapper)!
    let hit_visible_empty = find_clicked_button(
        &visible_drawer_root,
        &visible_root_node,
        Point::new(300.0, 500.0),
    );
    assert_eq!(hit_visible_empty, None);
}

#[test]
fn test_transformed_button_hitbox() {
    let style = Style {
        transform: sniffer_core::style::Transform {
            translate_y: 200.0,
            ..sniffer_core::style::Transform::default()
        },
        ..Style::default()
    };

    let btn = Element::Container {
        id: Some("btn_shifted".to_string()),
        style,
        children: vec![],
    };
    let btn_node = LayoutNode {
        element: btn.clone(),
        rect: Rect::new(50.0, 50.0, 100.0, 100.0),
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

    // Point at original position (50, 50) -> should NOT hit because button is translated down by 200px
    let hit_orig = find_clicked_button(&root, &root_node, Point::new(60.0, 60.0));
    assert_eq!(hit_orig, None);

    // Point at translated position (50, 250) -> should hit!
    let hit_shifted = find_clicked_button(&root, &root_node, Point::new(60.0, 260.0));
    assert!(hit_shifted.is_some());
    let (id_hash, rect) = hit_shifted.unwrap();
    assert_eq!(id_hash, sniffer_core::ui::widget::fnv1a(b"btn_shifted"));
    assert_eq!(rect.y, 250.0);
}
