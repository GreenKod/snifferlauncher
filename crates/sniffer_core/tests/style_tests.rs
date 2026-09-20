use sniffer_core::style::{Style, StyleOverride};
use sniffer_core::types::Element;

#[test]
fn test_style_defaults_include_elevation_and_shadow_and_gradient() {
    let style = Style::default();
    assert_eq!(style.elevation, 0.0);
    assert_eq!(style.shadow_blur, 0.0);
    assert_eq!(style.shadow_spread, 0.0);
    assert_eq!(style.shadow_offset_y, 0.0);
    assert_eq!(style.shadow_color, None);
    assert_eq!(style.border_gradient, None);
}

#[test]
fn test_style_builder_elevation_and_shadows() {
    let style = Style::builder()
        .elevation(8.0)
        .shadow_blur(16.0)
        .shadow_spread(2.0)
        .shadow_offset_y(4.0)
        .shadow_color(0x6600_0000)
        .border_gradient(0xFFFF_0000, 0xFF00_00FF)
        .build();

    assert_eq!(style.elevation, 8.0);
    assert_eq!(style.shadow_blur, 16.0);
    assert_eq!(style.shadow_spread, 2.0);
    assert_eq!(style.shadow_offset_y, 4.0);
    assert_eq!(style.shadow_color, Some(0x6600_0000));
    assert_eq!(style.border_gradient, Some((0xFFFF_0000, 0xFF00_00FF)));
}

#[test]
fn test_style_override_applies_elevation_and_border_gradient() {
    let base = Style::default();
    let style_override = StyleOverride {
        elevation: Some(12.0),
        shadow_blur: Some(24.0),
        shadow_spread: Some(4.0),
        shadow_color: Some(0x8000_0000),
        border_gradient: Some((0xFF11_2233, 0xFF44_5566)),
        ..Default::default()
    };

    let resolved = style_override.apply(base);
    assert_eq!(resolved.elevation, 12.0);
    assert_eq!(resolved.shadow_blur, 24.0);
    assert_eq!(resolved.shadow_spread, 4.0);
    assert_eq!(resolved.shadow_color, Some(0x8000_0000));
    assert_eq!(resolved.border_gradient, Some((0xFF11_2233, 0xFF44_5566)));
}

#[test]
fn test_style_serde_json_roundtrip_and_dimension_strings() {
    let json_input = r#"{
        "elevation": "10px",
        "shadow_blur": 15.5,
        "shadow_spread": "3px",
        "shadow_offset_y": 5.0,
        "shadow_color": 1711276032,
        "border_gradient": [4278190080, 4294967295]
    }"#;

    let style: Style = serde_json::from_str(json_input).expect("parse json");
    assert_eq!(style.elevation, 10.0);
    assert_eq!(style.shadow_blur, 15.5);
    assert_eq!(style.shadow_spread, 3.0);
    assert_eq!(style.shadow_offset_y, 5.0);
    assert_eq!(style.shadow_color, Some(1711276032));
    assert_eq!(style.border_gradient, Some((4278190080, 4294967295)));

    // Serialize back and verify
    let serialized = serde_json::to_string(&style).expect("serialize");
    let deserialized: Style = serde_json::from_str(&serialized).expect("roundtrip deserialize");
    assert_eq!(deserialized.elevation, 10.0);
    assert_eq!(deserialized.shadow_blur, 15.5);
    assert_eq!(deserialized.shadow_spread, 3.0);
    assert_eq!(deserialized.shadow_offset_y, 5.0);
    assert_eq!(deserialized.shadow_color, Some(1711276032));
    assert_eq!(deserialized.border_gradient, Some((4278190080, 4294967295)));
}

#[test]
fn test_element_mutate_style_elevation_and_shadow() {
    let mut element = Element::Container {
        id: Some("card_1".to_string()),
        style: Style::default(),
        children: Vec::new(),
    };

    assert!(element.mutate_style("card_1", "elevation", "6.5"));
    assert!(element.mutate_style("card_1", "shadow-blur", "12.0"));
    assert!(element.mutate_style("card_1", "shadow-spread", "2.0"));
    assert!(element.mutate_style("card_1", "shadow-offset-y", "4.0"));
    assert!(element.mutate_style("card_1", "shadow-color", "#33000000"));

    if let Element::Container { style, .. } = element {
        assert_eq!(style.elevation, 6.5);
        assert_eq!(style.shadow_blur, 12.0);
        assert_eq!(style.shadow_spread, 2.0);
        assert_eq!(style.shadow_offset_y, 4.0);
        assert_eq!(style.shadow_color, Some(0x3300_0000));
    } else {
        panic!("expected container");
    }
}
