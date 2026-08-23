#![allow(clippy::pedantic, clippy::nursery)]

pub mod atlas;
pub mod discovery;
pub mod rasterizer;
pub mod upload;

pub use atlas::{FontAtlas, GlyphInfo, build_font_atlas, estimate_text_width};
pub use discovery::discover_system_fonts;
