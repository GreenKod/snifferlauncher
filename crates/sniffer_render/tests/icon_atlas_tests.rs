use sniffer_render::glow::textures::{AtlasRegion, IconAtlas};
use std::num::NonZeroU32;

fn dummy_texture(id: u32) -> glow::Texture {
    glow::NativeTexture(NonZeroU32::new(id).unwrap())
}

#[test]
fn test_icon_atlas_can_pack_constraints() {
    let atlas = IconAtlas::new_with_texture(dummy_texture(1), 2048, 2048);
    assert!(atlas.can_pack(48, 48));
    assert!(atlas.can_pack(96, 96));
    assert!(atlas.can_pack(192, 192));
    assert!(atlas.can_pack(256, 256));

    // Exceeds max icon dimension
    assert!(!atlas.can_pack(257, 100));
    assert!(!atlas.can_pack(100, 300));
    assert!(!atlas.can_pack(1080, 1920));

    // Zero dimensions
    assert!(!atlas.can_pack(0, 50));
    assert!(!atlas.can_pack(50, 0));
}

#[test]
fn test_icon_atlas_shelf_allocation_and_uv_coords() {
    let mut atlas = IconAtlas::new_with_texture(dummy_texture(1), 1024, 1024);
    assert_eq!(atlas.vram_bytes(), 1024 * 1024 * 4);
    assert!(atlas.is_empty());
    assert_eq!(atlas.len(), 0);

    let region1 = atlas.allocate_slot(64, 64).expect("slot 1 fits");
    assert_eq!(
        region1,
        AtlasRegion {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            uv_rect: [0.0, 0.0, 64.0 / 1024.0, 64.0 / 1024.0]
        }
    );

    let region2 = atlas.allocate_slot(64, 64).expect("slot 2 fits");
    // With 1px padding, x should be 65
    assert_eq!(region2.x, 65);
    assert_eq!(region2.y, 0);
    assert_eq!(region2.width, 64);
    assert_eq!(region2.height, 64);
    assert!((region2.uv_rect[0] - 65.0 / 1024.0).abs() < 1e-5);
    assert!((region2.uv_rect[2] - 129.0 / 1024.0).abs() < 1e-5);
}

#[test]
fn test_icon_atlas_shelf_wrap_and_next_row() {
    // Atlas width of 200: can hold two 96px icons (96 + 1 = 97, 97*2 = 194 <= 200)
    let mut atlas = IconAtlas::new_with_texture(dummy_texture(1), 200, 500);

    let slot1 = atlas.allocate_slot(96, 96).expect("slot 1 fits");
    assert_eq!(slot1.x, 0);
    assert_eq!(slot1.y, 0);

    let slot2 = atlas.allocate_slot(96, 96).expect("slot 2 fits");
    assert_eq!(slot2.x, 97);
    assert_eq!(slot2.y, 0);

    // Third icon cannot fit on row 0 (97 + 97 = 194; 194 + 97 = 291 > 200)
    // Must wrap to row 1 at y = 97
    let slot3 = atlas
        .allocate_slot(96, 96)
        .expect("slot 3 fits on next shelf");
    assert_eq!(slot3.x, 0);
    assert_eq!(slot3.y, 97);
    assert_eq!(slot3.width, 96);
    assert_eq!(slot3.height, 96);
}

#[test]
fn test_icon_atlas_out_of_space() {
    // Small atlas that can only hold a single 64x64 item
    let mut atlas = IconAtlas::new_with_texture(dummy_texture(1), 100, 100);

    let slot1 = atlas.allocate_slot(64, 64).expect("slot 1 fits");
    assert_eq!(slot1.x, 0);
    assert_eq!(slot1.y, 0);

    // Second slot cannot fit on row 0 (65 + 65 = 130 > 100), and cannot fit on row 1 (65 + 65 = 130 > 100)
    let slot2 = atlas.allocate_slot(64, 64);
    assert!(slot2.is_none());
}

#[test]
fn test_icon_atlas_clear_and_reset() {
    let mut atlas = IconAtlas::new_with_texture(dummy_texture(1), 512, 512);

    atlas.regions.insert(
        "app_1".to_string(),
        AtlasRegion {
            x: 0,
            y: 0,
            width: 48,
            height: 48,
            uv_rect: [0.0, 0.0, 48.0 / 512.0, 48.0 / 512.0],
        },
    );
    atlas.current_x = 49;
    atlas.current_y = 0;
    atlas.shelf_height = 49;

    assert!(atlas.contains("app_1"));
    assert_eq!(atlas.len(), 1);

    atlas.clear();
    assert!(!atlas.contains("app_1"));
    assert!(atlas.is_empty());
    assert_eq!(atlas.current_x, 0);
    assert_eq!(atlas.current_y, 0);
    assert_eq!(atlas.shelf_height, 0);
}
