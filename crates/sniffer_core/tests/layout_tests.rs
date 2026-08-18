use sniffer_core::layout::calculate_layout;
use sniffer_core::math::Size;
use sniffer_core::style::{Dimension, FlexDirection, Style};
use sniffer_core::types::Element;

#[test]
fn test_basic_container_layout() {
    let child1 = Element::Container {
        id: Some("child_1".to_string()),
        style: Style {
            width: Dimension::Pixels(100.0),
            height: Dimension::Pixels(50.0),
            background_color: Some(0xFF00_00FF),
            ..Default::default()
        },
        children: vec![],
    };

    let child2 = Element::Container {
        id: Some("child_2".to_string()),
        style: Style {
            width: Dimension::Pixels(100.0),
            height: Dimension::Pixels(50.0),
            background_color: Some(0x00FF_00FF),
            ..Default::default()
        },
        children: vec![],
    };

    let root = Element::Container {
        id: Some("root".to_string()),
        style: Style {
            width: Dimension::Pixels(500.0),
            height: Dimension::Pixels(500.0),
            flex_direction: FlexDirection::Column,
            gap: 10.0,
            ..Default::default()
        },
        children: vec![child1, child2],
    };

    let layout = calculate_layout(&root, Size::new(500.0, 500.0), 0.0, 0.0);

    assert_eq!(layout.rect.width, 500.0);
    assert_eq!(layout.rect.height, 500.0);
    assert_eq!(layout.children.len(), 2);

    let c1 = &layout.children[0];
    assert_eq!(c1.rect.x, 0.0);
    assert_eq!(c1.rect.y, 0.0);
    assert_eq!(c1.rect.width, 100.0);
    assert_eq!(c1.rect.height, 50.0);

    let c2 = &layout.children[1];
    assert_eq!(c2.rect.x, 0.0);
    assert_eq!(c2.rect.y, 60.0); // 50.0 + 10.0 gap
    assert_eq!(c2.rect.width, 100.0);
    assert_eq!(c2.rect.height, 50.0);
}

#[test]
fn test_row_flex_layout() {
    let c1 = Element::Container {
        id: Some("col1".to_string()),
        style: Style {
            width: Dimension::Pixels(80.0),
            height: Dimension::Pixels(80.0),
            ..Default::default()
        },
        children: vec![],
    };

    let c2 = Element::Container {
        id: Some("col2".to_string()),
        style: Style {
            width: Dimension::Pixels(80.0),
            height: Dimension::Pixels(80.0),
            ..Default::default()
        },
        children: vec![],
    };

    let root = Element::Container {
        id: Some("row_root".to_string()),
        style: Style {
            width: Dimension::Pixels(400.0),
            height: Dimension::Pixels(200.0),
            flex_direction: FlexDirection::Row,
            gap: 20.0,
            ..Default::default()
        },
        children: vec![c1, c2],
    };

    let layout = calculate_layout(&root, Size::new(400.0, 200.0), 10.0, 20.0);
    assert_eq!(layout.rect.x, 10.0);
    assert_eq!(layout.rect.y, 20.0);

    let child_1 = &layout.children[0];
    assert_eq!(child_1.rect.x, 10.0);
    assert_eq!(child_1.rect.y, 20.0);

    let child_2 = &layout.children[1];
    assert_eq!(child_2.rect.x, 110.0); // 10.0 + 80.0 + 20.0 gap
    assert_eq!(child_2.rect.y, 20.0);
}
