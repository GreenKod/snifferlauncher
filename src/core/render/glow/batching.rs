use super::GlowRenderer;
use crate::core::render::math::geometry::Rect;
use glow::HasContext;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QuadInstanceData {
    pub rect_pos: [f32; 2],
    pub rect_size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
    pub border_width: f32,
    pub border_color: [f32; 4],
    pub is_circle: f32,
    pub is_shadow: f32,
    pub shadow_blur: f32,
    pub is_gradient: f32,
    pub color_bottom: [f32; 4],
    pub shape_size: [f32; 2],
}

pub struct QuadBatch {
    pub instances: Vec<QuadInstanceData>,
    pub max_capacity: usize,
}

impl Default for QuadBatch {
    fn default() -> Self {
        Self {
            instances: Vec::with_capacity(128),
            max_capacity: 512,
        }
    }
}

pub(crate) fn unpack_color(color: u32) -> [f32; 4] {
    let a =
        f32::from(u8::try_from((color >> 24) & 0xff).expect("alpha channel fits in u8")) / 255.0;
    let r = f32::from(u8::try_from((color >> 16) & 0xff).expect("red channel fits in u8")) / 255.0;
    let g = f32::from(u8::try_from((color >> 8) & 0xff).expect("green channel fits in u8")) / 255.0;
    let b = f32::from(u8::try_from(color & 0xff).expect("blue channel fits in u8")) / 255.0;
    let a = if a == 0.0 && color != 0 { 1.0 } else { a };
    [r, g, b, a]
}

impl GlowRenderer {
    pub(crate) fn draw_rect_impl(
        &mut self,
        rect: Rect,
        color: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        self.draw_rect_gradient_impl(rect, color, color, radius, border_width, border_color);
    }

    pub(crate) fn draw_rect_gradient_impl(
        &mut self,
        rect: Rect,
        color_top: u32,
        color_bottom: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        let mut col_top = unpack_color(color_top);
        let mut col_bot = unpack_color(color_bottom);
        col_top[3] *= self.global_alpha;
        col_bot[3] *= self.global_alpha;

        let mut b_col = unpack_color(border_color.unwrap_or(0));
        b_col[3] *= self.global_alpha;

        let is_gradient = if color_top == color_bottom {
            0.0f32
        } else {
            1.0f32
        };

        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vertex_array));

            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_border_w = self
                .gl
                .get_uniform_location(self.shape_program, "u_border_width");
            let loc_border_col = self
                .gl
                .get_uniform_location(self.shape_program, "u_border_color");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");
            let loc_is_gradient = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_gradient");
            let loc_color_bot = self
                .gl
                .get_uniform_location(self.shape_program, "u_color_bottom");
            let loc_shape_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_shape_size");

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl.uniform_2_f32(loc_rect_pos.as_ref(), rect.x, rect.y);
            self.gl
                .uniform_2_f32(loc_rect_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_2_f32(loc_shape_size.as_ref(), rect.width, rect.height);
            self.gl.uniform_4_f32(
                loc_color.as_ref(),
                col_top[0],
                col_top[1],
                col_top[2],
                col_top[3],
            );
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius);
            self.gl.uniform_1_f32(loc_border_w.as_ref(), border_width);
            self.gl.uniform_4_f32(
                loc_border_col.as_ref(),
                b_col[0],
                b_col[1],
                b_col[2],
                b_col[3],
            );
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 0.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 0.0);
            self.gl.uniform_1_f32(loc_is_gradient.as_ref(), is_gradient);
            self.gl.uniform_4_f32(
                loc_color_bot.as_ref(),
                col_bot[0],
                col_bot[1],
                col_bot[2],
                col_bot[3],
            );

            let loc_transform = self
                .gl
                .get_uniform_location(self.shape_program, "u_transform");
            let t = self.transform_stack.last().unwrap();
            self.gl
                .uniform_matrix_3_f32_slice(loc_transform.as_ref(), false, t);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    pub(crate) fn draw_shadow_impl(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32) {
        let mut col = unpack_color(color);
        col[3] *= self.global_alpha;
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vertex_array));

            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");
            let loc_shadow_blur = self
                .gl
                .get_uniform_location(self.shape_program, "u_shadow_blur");
            let loc_shape_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_shape_size");

            let blur = spread * 1.5;
            let padding = blur * 2.0;

            let shadow_rect = Rect {
                x: rect.x - spread - padding,
                y: rect.y + offset_y - spread - padding,
                width: spread.mul_add(2.0, rect.width) + padding * 2.0,
                height: spread.mul_add(2.0, rect.height) + padding * 2.0,
            };

            let shape_size_x = spread.mul_add(2.0, rect.width);
            let shape_size_y = spread.mul_add(2.0, rect.height);

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl
                .uniform_2_f32(loc_rect_pos.as_ref(), shadow_rect.x, shadow_rect.y);
            self.gl.uniform_2_f32(
                loc_rect_size.as_ref(),
                shadow_rect.width,
                shadow_rect.height,
            );
            self.gl
                .uniform_2_f32(loc_shape_size.as_ref(), shape_size_x, shape_size_y);
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius + spread);
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 0.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 1.0);
            self.gl.uniform_1_f32(loc_shadow_blur.as_ref(), blur);

            let loc_transform = self
                .gl
                .get_uniform_location(self.shape_program, "u_transform");
            let t = self.transform_stack.last().unwrap();
            self.gl
                .uniform_matrix_3_f32_slice(loc_transform.as_ref(), false, t);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    pub(crate) fn draw_circle_impl(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        let mut col = unpack_color(color);
        col[3] *= self.global_alpha;
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vertex_array));

            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");
            let loc_shape_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_shape_size");

            let rect = Rect {
                x: cx - radius,
                y: cy - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            };

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl.uniform_2_f32(loc_rect_pos.as_ref(), rect.x, rect.y);
            self.gl
                .uniform_2_f32(loc_rect_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_2_f32(loc_shape_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius);
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 1.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 0.0);

            let loc_transform = self
                .gl
                .get_uniform_location(self.shape_program, "u_transform");
            let t = self.transform_stack.last().unwrap();
            self.gl
                .uniform_matrix_3_f32_slice(loc_transform.as_ref(), false, t);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}
