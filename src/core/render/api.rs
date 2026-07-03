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
    fn draw_rect_gradient(
        &mut self,
        rect: Rect,
        color_top: u32,
        color_bottom: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    );
    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32);
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32);
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32);
    fn begin_frame(&mut self, width: f32, height: f32);
    fn end_frame(&mut self);
    fn set_clip_rect(&mut self, rect: Rect);
    fn clear_clip_rect(&mut self);
    fn set_global_alpha(&mut self, alpha: f32);
    fn load_image(&mut self, id: &str, rgba_pixels: &[u8], width: u32, height: u32);
    fn draw_image(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: crate::core::style::ObjectFit,
    );
    fn measure_text(&self, text: &str, size: f32) -> f32;
}
