use super::{GlowRenderer, batching, f32_to_i32, text};
use crate::text::font_atlas;
use glow::HasContext;
use sniffer_core::math::Rect;
use sniffer_core::render::Renderer;

impl Renderer for GlowRenderer {
    fn clear(&mut self, color: u32) {
        let col = batching::unpack_color(color);
        unsafe {
            self.gl.clear_color(col[0], col[1], col[2], col[3]);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    fn draw_rect(
        &mut self,
        rect: Rect,
        color: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        self.draw_rect_impl(rect, color, radius, border_width, border_color);
    }

    fn draw_rect_gradient(
        &mut self,
        rect: Rect,
        color_top: u32,
        color_bottom: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        self.draw_rect_gradient_impl(
            rect,
            color_top,
            color_bottom,
            radius,
            border_width,
            border_color,
        );
    }

    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32) {
        self.draw_shadow_impl(rect, radius, offset_y, spread, color);
    }

    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        self.draw_circle_impl(cx, cy, radius, color);
    }

    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32) {
        text::draw_text_impl(self, text, x, y, size, color);
    }

    fn begin_frame(&mut self, width: f32, height: f32) {
        self.resolution = (width, height);
        self.texture_cache.current_frame = self.texture_cache.current_frame.wrapping_add(1);
        unsafe {
            self.gl
                .viewport(0, 0, f32_to_i32(width), f32_to_i32(height));
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
    }

    fn end_frame(&mut self) {}

    fn set_clip_rect(&mut self, rect: Rect) {
        unsafe {
            self.gl.enable(glow::SCISSOR_TEST);
            let y = self.resolution.1 - rect.y - rect.height;
            self.gl.scissor(
                f32_to_i32(rect.x),
                f32_to_i32(y),
                f32_to_i32(rect.width),
                f32_to_i32(rect.height),
            );
        }
    }

    fn clear_clip_rect(&mut self) {
        unsafe {
            self.gl.disable(glow::SCISSOR_TEST);
        }
    }

    fn push_clip_rect(&mut self, rect: Rect, radius: f32) {
        let current = if let Some(&(cur_rect, _)) = self.clip_stack.last() {
            let cx = cur_rect.x.max(rect.x);
            let cy = cur_rect.y.max(rect.y);
            let cw = (cur_rect.x + cur_rect.width).min(rect.x + rect.width) - cx;
            let ch = (cur_rect.y + cur_rect.height).min(rect.y + rect.height) - cy;
            Rect::new(cx, cy, cw.max(0.0), ch.max(0.0))
        } else {
            rect
        };
        self.clip_stack.push((current, radius));
        self.set_clip_rect(current);
    }

    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        if let Some(&(rect, _)) = self.clip_stack.last() {
            self.set_clip_rect(rect);
        } else {
            self.clear_clip_rect();
        }
    }

    fn push_transform(&mut self, cx: f32, cy: f32, scale: f32, rotate: f32, tx: f32, ty: f32) {
        let p = self
            .transform_stack
            .last()
            .copied()
            .unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

        let rot_rad = rotate.to_radians();
        let c = rot_rad.cos() * scale;
        let s = rot_rad.sin() * scale;

        let b0 = c;
        let b1 = s;
        let b3 = -s;
        let b4 = c;

        let b6 = cx + tx - (c * cx - s * cy);
        let b7 = cy + ty - (s * cx + c * cy);

        let res = [
            b1.mul_add(p[3], p[0] * b0),
            b1.mul_add(p[4], p[1] * b0),
            b1.mul_add(p[5], p[2] * b0),
            b4.mul_add(p[3], p[0] * b3),
            b4.mul_add(p[4], p[1] * b3),
            b4.mul_add(p[5], p[2] * b3),
            p[6] + b7.mul_add(p[3], p[0] * b6),
            p[7] + b7.mul_add(p[4], p[1] * b6),
            p[8] + b7.mul_add(p[5], p[2] * b6),
        ];

        self.transform_stack.push(res);
    }

    fn pop_transform(&mut self) {
        if self.transform_stack.len() > 1 {
            self.transform_stack.pop();
        }
    }

    fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha;
    }

    fn load_image(&mut self, id: &str, rgba_pixels: &[u8], width: u32, height: u32) {
        self.load_image_impl(id, rgba_pixels, width, height);
    }

    fn has_image(&self, id: &str) -> bool {
        self.has_image_impl(id)
    }

    fn load_wallpaper(&mut self, rgba_pixels: &[u8], width: u32, height: u32) {
        self.load_wallpaper_impl(rgba_pixels, width, height);
    }

    fn draw_wallpaper(&mut self, width: f32, height: f32) {
        self.draw_wallpaper_impl(width, height);
    }

    fn draw_image(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: sniffer_core::style::ObjectFit,
    ) {
        self.draw_image_impl(id, rect, radius, object_fit);
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.font_atlas.as_ref().map_or_else(
            || {
                let char_width = size;
                let gap = size * 0.1;
                #[allow(clippy::cast_precision_loss)]
                let count = text.chars().count() as f32;
                if count > 0.0 {
                    (char_width + gap).mul_add(count, -gap)
                } else {
                    0.0
                }
            },
            |atlas| font_atlas::estimate_text_width(atlas, text, size),
        )
    }

    fn text_ascent(&self, size: f32) -> f32 {
        self.font_atlas.as_ref().map_or(size * 0.75, |atlas| {
            let scale = size / atlas.rasterize_size;
            atlas.ascent * scale
        })
    }
}
