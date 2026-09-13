use super::GlowRenderer;
use glow::HasContext;
use sniffer_core::math::Rect;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct QuadInstanceData {
    pub rect_pos: [f32; 2],
    pub rect_size: [f32; 2],
    pub color: [f32; 4],
    pub border_color: [f32; 4],
    pub color_bottom: [f32; 4],
    pub shape_size: [f32; 2],
    pub is_circle: f32,
    pub is_shadow: f32,
    pub radius: f32,
    pub border_width: f32,
    pub shadow_blur: f32,
    pub is_gradient: f32,
}

pub struct QuadBatch {
    pub instances: Vec<QuadInstanceData>,
    pub max_capacity: usize,
}

impl QuadBatch {
    pub const DEFAULT_MAX_CAPACITY: usize = 512;
    pub const INITIAL_CAPACITY: usize = 128;

    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(Self::INITIAL_CAPACITY.min(max_capacity)),
            max_capacity,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[inline]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.instances.len() >= self.max_capacity
    }

    #[inline]
    pub fn push_instance(&mut self, instance: QuadInstanceData) -> bool {
        if self.is_full() {
            return false;
        }
        self.instances.push(instance);
        true
    }

    #[inline]
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[QuadInstanceData] {
        &self.instances
    }

    /// Returns the raw byte representation of instances for OpenGL buffer upload.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.instances.as_ptr().cast::<u8>(),
                self.instances.len() * std::mem::size_of::<QuadInstanceData>(),
            )
        }
    }
}

impl Default for QuadBatch {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_CAPACITY)
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
    pub fn flush_shapes(&mut self) {
        if self.shape_batch.is_empty() {
            return;
        }

        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.ensure_shape_instance_vao();

            let u = &self.shape_uniforms;
            self.gl.uniform_1_i32(u.u_instanced.as_ref(), 1);
            self.gl.uniform_2_f32(
                u.u_resolution.as_ref(),
                self.resolution.0,
                self.resolution.1,
            );

            if let Some(t) = self.transform_stack.last() {
                self.gl
                    .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
            }

            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.shape_instance_vbo));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                self.shape_batch.as_bytes(),
                glow::DYNAMIC_DRAW,
            );

            let count = i32::try_from(self.shape_batch.len()).expect("instance count fits in i32");
            self.gl
                .draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, count);

            self.shape_batch.clear();
        }
    }

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
        if self.shape_batch.is_full() {
            self.flush_shapes();
        }

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

        let instance = QuadInstanceData {
            rect_pos: [rect.x, rect.y],
            rect_size: [rect.width, rect.height],
            color: col_top,
            border_color: b_col,
            color_bottom: col_bot,
            shape_size: [rect.width, rect.height],
            is_circle: 0.0,
            is_shadow: 0.0,
            radius,
            border_width,
            shadow_blur: 0.0,
            is_gradient,
        };

        let _ = self.shape_batch.push_instance(instance);
    }

    pub(crate) fn draw_shadow_impl(
        &mut self,
        rect: Rect,
        radius: f32,
        offset_y: f32,
        spread: f32,
        color: u32,
    ) {
        self.flush_shapes();

        let mut col = unpack_color(color);
        col[3] *= self.global_alpha;
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.ensure_quad_vao();

            let u = &self.shape_uniforms;
            self.gl.uniform_1_i32(u.u_instanced.as_ref(), 0);

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

            self.gl.uniform_2_f32(
                u.u_resolution.as_ref(),
                self.resolution.0,
                self.resolution.1,
            );
            self.gl
                .uniform_2_f32(u.u_rect_pos.as_ref(), shadow_rect.x, shadow_rect.y);
            self.gl.uniform_2_f32(
                u.u_rect_size.as_ref(),
                shadow_rect.width,
                shadow_rect.height,
            );
            self.gl
                .uniform_2_f32(u.u_shape_size.as_ref(), shape_size_x, shape_size_y);
            self.gl
                .uniform_4_f32(u.u_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(u.u_radius.as_ref(), radius + spread);
            self.gl.uniform_1_f32(u.u_is_circle.as_ref(), 0.0);
            self.gl.uniform_1_f32(u.u_is_shadow.as_ref(), 1.0);
            self.gl.uniform_1_f32(u.u_shadow_blur.as_ref(), blur);

            let Some(t) = self.transform_stack.last() else {
                crate::dev_err!("transform_stack empty in draw_shadow — missing push_transform");
                return;
            };
            self.gl
                .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    pub(crate) fn draw_circle_impl(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        self.flush_shapes();

        let mut col = unpack_color(color);
        col[3] *= self.global_alpha;
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.ensure_quad_vao();

            let u = &self.shape_uniforms;
            self.gl.uniform_1_i32(u.u_instanced.as_ref(), 0);

            let rect = Rect {
                x: cx - radius,
                y: cy - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            };

            self.gl.uniform_2_f32(
                u.u_resolution.as_ref(),
                self.resolution.0,
                self.resolution.1,
            );
            self.gl.uniform_2_f32(u.u_rect_pos.as_ref(), rect.x, rect.y);
            self.gl
                .uniform_2_f32(u.u_rect_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_2_f32(u.u_shape_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_4_f32(u.u_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(u.u_radius.as_ref(), radius);
            self.gl.uniform_1_f32(u.u_is_circle.as_ref(), 1.0);
            self.gl.uniform_1_f32(u.u_is_shadow.as_ref(), 0.0);

            let Some(t) = self.transform_stack.last() else {
                crate::dev_err!("transform_stack empty in draw_circle — missing push_transform");
                return;
            };
            self.gl
                .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quad_instance_data_layout() {
        assert_eq!(std::mem::size_of::<QuadInstanceData>(), 96);
        assert_eq!(std::mem::align_of::<QuadInstanceData>(), 4);

        let data = QuadInstanceData::default();
        let base = &raw const data as usize;

        // Verify that 6 attribute blocks (each vec4 = 16 bytes) align perfectly:
        // 1. a_bounds: rect_pos (8B) + rect_size (8B) = 16B at offset 0
        assert_eq!(&raw const data.rect_pos as usize - base, 0);
        assert_eq!(&raw const data.rect_size as usize - base, 8);

        // 2. a_color: color (16B) at offset 16
        assert_eq!(&raw const data.color as usize - base, 16);

        // 3. a_border_color: border_color (16B) at offset 32
        assert_eq!(&raw const data.border_color as usize - base, 32);

        // 4. a_color_bottom: color_bottom (16B) at offset 48
        assert_eq!(&raw const data.color_bottom as usize - base, 48);

        // 5. a_shape_info: shape_size (8B) + is_circle (4B) + is_shadow (4B) = 16B at offset 64
        assert_eq!(&raw const data.shape_size as usize - base, 64);
        assert_eq!(&raw const data.is_circle as usize - base, 72);
        assert_eq!(&raw const data.is_shadow as usize - base, 76);

        // 6. a_params: radius (4B) + border_width (4B) + shadow_blur (4B) + is_gradient (4B) = 16B at offset 80
        assert_eq!(&raw const data.radius as usize - base, 80);
        assert_eq!(&raw const data.border_width as usize - base, 84);
        assert_eq!(&raw const data.shadow_blur as usize - base, 88);
        assert_eq!(&raw const data.is_gradient as usize - base, 92);
    }

    #[test]
    fn test_quad_batch_management() {
        let mut batch = QuadBatch::new(4);
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert!(!batch.is_full());
        assert_eq!(batch.as_slice().len(), 0);

        let inst = QuadInstanceData::default();
        assert!(batch.push_instance(inst));
        assert!(batch.push_instance(inst));
        assert!(batch.push_instance(inst));
        assert!(batch.push_instance(inst));
        assert_eq!(batch.len(), 4);
        assert!(batch.is_full());

        // Push beyond capacity fails
        assert!(!batch.push_instance(inst));
        assert_eq!(batch.len(), 4);

        // Raw byte representation size check: 4 instances * 96 bytes = 384 bytes
        assert_eq!(batch.as_bytes().len(), 4 * 96);

        // Clear preserves buffer allocation
        batch.clear();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert!(!batch.is_full());
        assert!(batch.instances.capacity() >= 4);
    }

    #[test]
    fn test_quad_instance_data_construction() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let color_top = 0xFFFF_0000;
        let color_bottom = 0xFF00_00FF;
        let radius = 8.0;
        let border_width = 2.0;
        let border_color = 0xFF00_FF00;

        let col_top = unpack_color(color_top);
        let col_bot = unpack_color(color_bottom);
        let b_col = unpack_color(border_color);

        let instance = QuadInstanceData {
            rect_pos: [rect.x, rect.y],
            rect_size: [rect.width, rect.height],
            color: col_top,
            border_color: b_col,
            color_bottom: col_bot,
            shape_size: [rect.width, rect.height],
            is_circle: 0.0,
            is_shadow: 0.0,
            radius,
            border_width,
            shadow_blur: 0.0,
            is_gradient: 1.0,
        };

        assert_eq!(instance.rect_pos, [10.0, 20.0]);
        assert_eq!(instance.rect_size, [100.0, 50.0]);
        assert_eq!(instance.radius, 8.0);
        assert_eq!(instance.border_width, 2.0);
        assert_eq!(instance.is_gradient, 1.0);
        assert_eq!(instance.is_circle, 0.0);
        assert_eq!(instance.is_shadow, 0.0);
    }
}
