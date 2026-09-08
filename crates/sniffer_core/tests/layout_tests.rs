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

#[test]
fn test_padding_on_wrap_container() {
    use sniffer_core::style::{FlexWrap, JustifyContent, RectOffset};

    let c1 = Element::Container {
        id: Some("card1".to_string()),
        style: Style {
            width: Dimension::Pixels(200.0),
            height: Dimension::Pixels(200.0),
            ..Default::default()
        },
        children: vec![],
    };

    let root = Element::Container {
        id: Some("grid".to_string()),
        style: Style {
            width: Dimension::Pixels(1000.0),
            height: Dimension::Pixels(1800.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Start,
            padding: RectOffset {
                top: 276.0,
                right: 70.0,
                bottom: 24.0,
                left: 70.0,
            },
            ..Default::default()
        },
        children: vec![c1],
    };

    let layout = calculate_layout(&root, Size::new(1000.0, 1800.0), 0.0, 0.0);
    println!("Root rect: {:?}", layout.rect);
    println!("Child 1 rect: {:?}", layout.children[0].rect);
    assert_eq!(layout.children[0].rect.y, 276.0);
}

#[test]
fn test_full_hierarchy_card_position() {
    use sniffer_core::style::{
        AlignItems, FlexDirection, FlexWrap, JustifyContent, Position, RectOffset,
    };

    // 1. Card
    let card = Element::Container {
        id: Some("p0_card_0".to_string()),
        style: Style {
            width: Dimension::Pixels(210.6),
            height: Dimension::Pixels(216.0),
            ..Default::default()
        },
        children: vec![],
    };

    // 2. Page Grid
    let page_grid = Element::Container {
        id: Some("page_grid_0".to_string()),
        style: Style {
            width: Dimension::Pixels(1080.0),
            height: Dimension::Pixels(1776.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Start,
            padding: RectOffset {
                top: 62.4,
                right: 50.76,
                bottom: 24.0,
                left: 50.76,
            },
            column_gap: 47.52,
            row_gap: 72.0,
            ..Default::default()
        },
        children: vec![card],
    };

    // 3. App Grid Track
    let track = Element::Container {
        id: Some("app_grid_track".to_string()),
        style: Style {
            width: Dimension::Pixels(1080.0),
            height: Dimension::Pixels(1776.0),
            flex_direction: FlexDirection::Row,
            flex_shrink: 0.0,
            ..Default::default()
        },
        children: vec![page_grid],
    };

    // 4. ScrollView pager
    let pager = Element::ScrollView {
        id: Some("app_grid_pager".to_string()),
        style: Style {
            width: Dimension::Pixels(1080.0),
            height: Dimension::Pixels(1776.0),
            overflow_hidden: true,
            ..Default::default()
        },
        children: vec![track],
        scroll_x: 0.0,
        scroll_y: 0.0,
        scroll_sensitivity: None,
        dynamic_sensitivity: None,
        momentum_scrolling: None,
        capture_drag: None,
        snap_x: None,
        snap_y: None,
        rubber_band: None,
        page_count: None,
        on_snap: None,
    };

    // 5. App Grid Wrapper
    let wrapper = Element::Container {
        id: Some("app_grid_wrapper".to_string()),
        style: Style {
            width: Dimension::Pixels(1080.0),
            height: Dimension::Pixels(1776.0),
            position: Position::Relative,
            ..Default::default()
        },
        children: vec![pager],
    };

    // 6. Home Screen Layer
    let home = Element::Container {
        id: Some("home_screen_layer".to_string()),
        style: Style {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            position: Position::Relative,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            ..Default::default()
        },
        children: vec![wrapper],
    };

    // 7. Root
    let root = Element::Container {
        id: Some("root".to_string()),
        style: Style {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            position: Position::Relative,
            ..Default::default()
        },
        children: vec![home],
    };

    let layout = calculate_layout(&root, Size::new(1080.0, 2400.0), 0.0, 0.0);
    let card_node = &layout.children[0].children[0].children[0].children[0].children[0].children[0];
    println!(">>> FULL TREE Card rect: {:?}", card_node.rect);
    assert_eq!(card_node.rect.y, 62.0);
}
