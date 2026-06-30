use crate::core::render::math::geometry::Rect;

/// Platform-agnostic 2D rendering interface.
/// Implemented by `GlowRenderer` for both desktop (OpenGL 3.3) and Android (OpenGL ES 3.0).
pub trait Renderer {
    fn clear(&mut self, color: u32);
    fn draw_rect(
        &mut self,
        rect: Rect,
        color: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    );
    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32);
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32);
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32);
    fn begin_frame(&mut self, width: f32, height: f32);
    fn end_frame(&mut self);
}
