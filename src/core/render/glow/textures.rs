use super::GlowRenderer;
use crate::core::render::api::Renderer;
use crate::core::render::math::geometry::Rect;
use glow::HasContext;

pub struct TextureHandle {
    pub texture: glow::Texture,
    pub width: f32,
    pub height: f32,
    pub size_bytes: usize,
    pub last_used: u64,
}

pub struct LruTextureCache {
    pub textures: std::collections::HashMap<String, TextureHandle>,
    pub access_counter: u64,
    pub total_vram_bytes: usize,
    pub max_vram_bytes: usize,
    pub max_textures: usize,
}

impl Default for LruTextureCache {
    fn default() -> Self {
        Self {
            textures: std::collections::HashMap::new(),
            access_counter: 0,
            total_vram_bytes: 0,
            max_vram_bytes: 64 * 1024 * 1024,
            max_textures: 128,
        }
    }
}

impl GlowRenderer {
    pub(crate) fn load_image_impl(&mut self, id: &str, rgba_pixels: &[u8], width: u32, height: u32) {
        if self.texture_cache.textures.contains_key(id) {
            self.texture_cache.access_counter += 1;
            if let Some(handle) = self.texture_cache.textures.get_mut(id) {
                handle.last_used = self.texture_cache.access_counter;
            }
            return;
        }

        let new_size = (width as usize) * (height as usize) * 4;

        unsafe {
            while (self.texture_cache.total_vram_bytes + new_size > self.texture_cache.max_vram_bytes
                || self.texture_cache.textures.len() >= self.texture_cache.max_textures)
                && self.texture_cache.textures.len() > 1
            {
                let lru_key = self
                    .texture_cache
                    .textures
                    .iter()
                    .filter(|(k, _)| k.as_str() != "__system_wallpaper__")
                    .min_by_key(|(_, v)| v.last_used)
                    .map(|(k, _)| k.clone());

                if let Some(evict_key) = lru_key {
                    if let Some(removed) = self.texture_cache.textures.remove(&evict_key) {
                        self.gl.delete_texture(removed.texture);
                        self.texture_cache.total_vram_bytes = self
                            .texture_cache
                            .total_vram_bytes
                            .saturating_sub(removed.size_bytes);
                    }
                } else {
                    break;
                }
            }

            let tex = self.gl.create_texture().unwrap();
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA).expect("RGBA fits in i32"),
                i32::try_from(width).expect("width fits in i32"),
                i32::try_from(height).expect("height fits in i32"),
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba_pixels)),
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                i32::try_from(glow::LINEAR).unwrap(),
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                i32::try_from(glow::LINEAR).unwrap(),
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                i32::try_from(glow::CLAMP_TO_EDGE).unwrap(),
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                i32::try_from(glow::CLAMP_TO_EDGE).unwrap(),
            );

            self.texture_cache.access_counter += 1;
            self.texture_cache.total_vram_bytes += new_size;
            #[allow(clippy::cast_precision_loss)]
            self.texture_cache.textures.insert(
                id.to_string(),
                TextureHandle {
                    texture: tex,
                    width: width as f32,
                    height: height as f32,
                    size_bytes: new_size,
                    last_used: self.texture_cache.access_counter,
                },
            );
        }
    }

    pub(crate) fn has_image_impl(&self, id: &str) -> bool {
        self.texture_cache.textures.contains_key(id)
    }

    pub(crate) fn load_wallpaper_impl(&mut self, rgba_pixels: &[u8], width: u32, height: u32) {
        unsafe {
            if let Some(removed) = self.texture_cache.textures.remove("__system_wallpaper__") {
                self.gl.delete_texture(removed.texture);
                self.texture_cache.total_vram_bytes = self
                    .texture_cache
                    .total_vram_bytes
                    .saturating_sub(removed.size_bytes);
            }
        }
        self.load_image_impl("__system_wallpaper__", rgba_pixels, width, height);
    }

    pub(crate) fn draw_wallpaper_impl(&mut self, width: f32, height: f32) {
        if self.has_image_impl("__system_wallpaper__") {
            let full_rect = Rect::new(0.0, 0.0, width, height);
            self.draw_image(
                "__system_wallpaper__",
                full_rect,
                0.0,
                crate::core::style::ObjectFit::Cover,
            );
            self.draw_rect(
                full_rect,
                0x4000_0000,
                0.0,
                0.0,
                None,
            );
        }
    }

    pub(crate) fn draw_image_impl(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: crate::core::style::ObjectFit,
    ) {
        self.texture_cache.access_counter += 1;
        let current_access = self.texture_cache.access_counter;

        if let Some(handle) = self.texture_cache.textures.get_mut(id) {
            handle.last_used = current_access;
            let tex = handle.texture;
            let img_w = handle.width;
            let img_h = handle.height;

            let mut draw_rect = rect;
            let mut uv_scale = (1.0_f32, 1.0_f32);
            let mut uv_offset = (0.0_f32, 0.0_f32);

            let img_aspect = img_w / img_h;
            let rect_aspect = rect.width / rect.height;

            match object_fit {
                crate::core::style::ObjectFit::Fill => {}
                crate::core::style::ObjectFit::Contain => {
                    if img_aspect > rect_aspect {
                        let target_h = rect.width / img_aspect;
                        draw_rect.y = rect.y + (rect.height - target_h) / 2.0;
                        draw_rect.height = target_h;
                    } else {
                        let target_w = rect.height * img_aspect;
                        draw_rect.x = rect.x + (rect.width - target_w) / 2.0;
                        draw_rect.width = target_w;
                    }
                }
                crate::core::style::ObjectFit::Cover => {
                    if img_aspect > rect_aspect {
                        let scale_x = rect_aspect / img_aspect;
                        uv_scale.0 = scale_x;
                        uv_offset.0 = (1.0 - scale_x) / 2.0;
                    } else {
                        let scale_y = img_aspect / rect_aspect;
                        uv_scale.1 = scale_y;
                        uv_offset.1 = (1.0 - scale_y) / 2.0;
                    }
                }
            }

            unsafe {
                self.gl.use_program(Some(self.image_program));
                self.gl.bind_vertex_array(Some(self.quad_vertex_array));
                self.gl.active_texture(glow::TEXTURE0);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));

                let loc_res = self
                    .gl
                    .get_uniform_location(self.image_program, "u_resolution");
                let loc_pos = self
                    .gl
                    .get_uniform_location(self.image_program, "u_rect_pos");
                let loc_size = self
                    .gl
                    .get_uniform_location(self.image_program, "u_rect_size");
                let loc_uv_s = self
                    .gl
                    .get_uniform_location(self.image_program, "u_uv_scale");
                let loc_uv_o = self
                    .gl
                    .get_uniform_location(self.image_program, "u_uv_offset");
                let loc_radius = self.gl.get_uniform_location(self.image_program, "u_radius");
                let loc_alpha = self
                    .gl
                    .get_uniform_location(self.image_program, "u_global_alpha");

                self.gl
                    .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
                self.gl
                    .uniform_2_f32(loc_pos.as_ref(), draw_rect.x, draw_rect.y);
                self.gl
                    .uniform_2_f32(loc_size.as_ref(), draw_rect.width, draw_rect.height);
                self.gl
                    .uniform_2_f32(loc_uv_s.as_ref(), uv_scale.0, uv_scale.1);
                self.gl
                    .uniform_2_f32(loc_uv_o.as_ref(), uv_offset.0, uv_offset.1);
                self.gl.uniform_1_f32(loc_radius.as_ref(), radius);
                self.gl.uniform_1_f32(loc_alpha.as_ref(), self.global_alpha);

                let loc_transform = self
                    .gl
                    .get_uniform_location(self.image_program, "u_transform");
                let t = self.transform_stack.last().unwrap();
                self.gl
                    .uniform_matrix_3_f32_slice(loc_transform.as_ref(), false, t);
                self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }
        }
    }
}
