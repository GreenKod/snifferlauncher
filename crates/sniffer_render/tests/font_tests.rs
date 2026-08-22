use sniffer_render::text::font_atlas::{discover_system_fonts, estimate_text_width, FontAtlas, GlyphInfo};

#[test]
fn test_system_font_discovery() {
    let fonts = discover_system_fonts();
    // System font discovery should succeed without crashing
    println!("Discovered {} system/emoji fonts on host", fonts.len());
}

#[test]
fn test_font_atlas_text_width_with_turkish_and_emoji() {
    let dummy_tex = glow::NativeTexture(std::num::NonZeroU32::new(1).unwrap());
    let mut glyphs = std::collections::HashMap::new();

    // Insert dummy glyph info for ASCII and Turkish characters
    glyphs.insert('A', GlyphInfo {
        atlas_x: 0, atlas_y: 0, width: 20, height: 30,
        bearing_x: 0.0, bearing_y: 25.0, advance_width: 22.0, is_color: false,
    });
    glyphs.insert('ç', GlyphInfo {
        atlas_x: 24, atlas_y: 0, width: 20, height: 35,
        bearing_x: 0.0, bearing_y: 25.0, advance_width: 22.0, is_color: false,
    });
    glyphs.insert('ğ', GlyphInfo {
        atlas_x: 48, atlas_y: 0, width: 20, height: 35,
        bearing_x: 0.0, bearing_y: 30.0, advance_width: 22.0, is_color: false,
    });
    glyphs.insert('🔥', GlyphInfo {
        atlas_x: 72, atlas_y: 0, width: 32, height: 32,
        bearing_x: 0.0, bearing_y: 28.0, advance_width: 32.0, is_color: true,
    });

    let atlas = FontAtlas::new_mock(
        dummy_tex,
        glyphs,
        64.0,
        50.0,
        16.0,
    );

    let w_empty = estimate_text_width(&atlas, "", 16.0);
    assert_eq!(w_empty, 0.0);

    let w_text = estimate_text_width(&atlas, "A ç ğ 🔥", 64.0);
    // 22 (A) + 16 (space) + 22 (ç) + 16 (space) + 22 (ğ) + 16 (space) + 32 (🔥) = 146
    assert_eq!(w_text, 146.0);
}
