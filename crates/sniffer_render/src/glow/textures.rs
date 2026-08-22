use super::GlowRenderer;
use glow::HasContext;
use sniffer_core::math::Rect;
use sniffer_core::render_api::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrimLevel {
    /// Evict until under max_vram_bytes
    Normal,
    /// Evict 50% oldest unpinned textures under memory pressure
    Moderate,
    /// Evict all non-system textures
    Critical,
}

pub struct TextureHandle {
    pub texture: glow::Texture,
    pub width: f32,
    pub height: f32,
    pub size_bytes: usize,
    pub last_frame: u64,
    prev: Option<usize>,
    next: Option<usize>,
}

pub struct LruTextureCache {
    nodes: Vec<Option<TextureHandle>>,
    keys: Vec<Option<String>>,
    free_indices: Vec<usize>,
    map: std::collections::HashMap<String, usize>,
    head: Option<usize>, // Most recently used
    tail: Option<usize>, // Least recently used
    pub total_vram_bytes: usize,
    pub max_vram_bytes: usize,
    pub max_textures: usize,
    pub current_frame: u64,
}

impl Default for LruTextureCache {
    fn default() -> Self {
        Self {
            nodes: Vec::with_capacity(128),
            keys: Vec::with_capacity(128),
            free_indices: Vec::new(),
            map: std::collections::HashMap::with_capacity(128),
            head: None,
            tail: None,
            total_vram_bytes: 0,
            max_vram_bytes: 128 * 1024 * 1024, // 128 MB VRAM limit
            max_textures: 1024,
            current_frame: 0,
        }
    }
}

impl LruTextureCache {
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn detach(&mut self, idx: usize) {
        let (prev, next) = match self.nodes.get(idx).and_then(Option::as_ref) {
            Some(node) => (node.prev, node.next),
            None => return,
        };

        if let Some(p) = prev {
            if let Some(Some(p_node)) = self.nodes.get_mut(p) {
                p_node.next = next;
            }
        } else {
            self.head = next;
        }

        if let Some(n) = next {
            if let Some(Some(n_node)) = self.nodes.get_mut(n) {
                n_node.prev = prev;
            }
        } else {
            self.tail = prev;
        }

        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.prev = None;
            node.next = None;
        }
    }

    fn attach_head(&mut self, idx: usize) {
        let old_head = self.head;
        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.prev = None;
            node.next = old_head;
        }

        if let Some(h) = old_head {
            if let Some(Some(h_node)) = self.nodes.get_mut(h) {
                h_node.prev = Some(idx);
            }
        } else {
            self.tail = Some(idx);
        }

        self.head = Some(idx);
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TextureHandle> {
        let idx = *self.map.get(key)?;
        self.detach(idx);
        self.attach_head(idx);
        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.last_frame = self.current_frame;
            Some(node)
        } else {
            None
        }
    }

    pub fn pin(&mut self, key: &str, frame: u64) {
        if let Some(&idx) = self.map.get(key) {
            self.detach(idx);
            self.attach_head(idx);
            if let Some(Some(node)) = self.nodes.get_mut(idx) {
                node.last_frame = frame;
            }
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<TextureHandle> {
        let idx = self.map.remove(key)?;
        self.detach(idx);

        let handle = self.nodes.get_mut(idx)?.take()?;
        self.keys[idx] = None;
        self.free_indices.push(idx);
        Some(handle)
    }

    pub fn insert(
        &mut self,
        key: String,
        texture: glow::Texture,
        width: f32,
        height: f32,
        size_bytes: usize,
    ) -> Option<TextureHandle> {
        if let Some(&idx) = self.map.get(&key) {
            self.detach(idx);
            self.attach_head(idx);
            let old = self.nodes.get_mut(idx)?.take();
            self.nodes[idx] = Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            });
            // Re-link head
            self.detach(idx);
            self.attach_head(idx);
            return old;
        }

        let idx = if let Some(free_idx) = self.free_indices.pop() {
            self.nodes[free_idx] = Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            });
            self.keys[free_idx] = Some(key.clone());
            free_idx
        } else {
            let new_idx = self.nodes.len();
            self.nodes.push(Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            }));
            self.keys.push(Some(key.clone()));
            new_idx
        };

        self.map.insert(key, idx);
        self.attach_head(idx);
        None
    }

    /// Finds the least-recently used key for eviction, preferring unpinned textures.
    #[must_use]
    pub fn pop_lru_candidate(&self, skip_wallpaper: bool, current_frame: u64) -> Option<String> {
        let mut curr = self.tail;
        let mut fallback: Option<String> = None;

        while let Some(idx) = curr {
            if let (Some(Some(node)), Some(Some(key))) = (self.nodes.get(idx), self.keys.get(idx)) {
                if !skip_wallpaper || key.as_str() != "__system_wallpaper__" {
                    if node.last_frame < current_frame {
                        return Some(key.clone());
                    }
                    if fallback.is_none() {
                        fallback = Some(key.clone());
                    }
                }
                curr = node.prev;
            } else {
                break;
            }
        }

        fallback
    }
}

impl GlowRenderer {
    pub(crate) fn load_image_impl(
        &mut self,
        id: &str,
        rgba_pixels: &[u8],
        width: u32,
        height: u32,
    ) {
        let w_f32 = width as f32;
        let h_f32 = height as f32;
        let new_size = (width as usize) * (height as usize) * 4;

        if let Some(handle) = self.texture_cache.get_mut(id) {
            if (handle.width - w_f32).abs() < f32::EPSILON
                && (handle.height - h_f32).abs() < f32::EPSILON
            {
                return;
            }
        }

        if let Some(removed) = self.texture_cache.remove(id) {
            unsafe {
                self.gl.delete_texture(removed.texture);
            }
            self.texture_cache.total_vram_bytes = self
                .texture_cache
                .total_vram_bytes
                .saturating_sub(removed.size_bytes);
        }

        unsafe {
            let current_frame = self.texture_cache.current_frame;
            while (self.texture_cache.total_vram_bytes + new_size
                > self.texture_cache.max_vram_bytes
                || self.texture_cache.len() >= self.texture_cache.max_textures)
                && self.texture_cache.len() > 1
            {
                if let Some(evict_key) =
                    self.texture_cache.pop_lru_candidate(true, current_frame)
                {
                    if let Some(removed) = self.texture_cache.remove(&evict_key) {
                        self.gl.delete_texture(removed.texture);
                        self.texture_cache.total_vram_bytes = self
                            .texture_cache
                            .total_vram_bytes
                            .saturating_sub(removed.size_bytes);
                    } else {
                        break;
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
            if width > 256 || height > 256 {
                self.gl.generate_mipmap(glow::TEXTURE_2D);
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    i32::try_from(glow::LINEAR_MIPMAP_LINEAR).unwrap(),
                );
            } else {
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    i32::try_from(glow::LINEAR).unwrap(),
                );
            }
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

            self.texture_cache.total_vram_bytes += new_size;
            let old_tex = self.texture_cache.insert(
                id.to_string(),
                tex,
                w_f32,
                h_f32,
                new_size,
            );
            if let Some(old) = old_tex {
                self.gl.delete_texture(old.texture);
                self.texture_cache.total_vram_bytes = self
                    .texture_cache
                    .total_vram_bytes
                    .saturating_sub(old.size_bytes);
            }
        }
    }

    pub(crate) fn has_image_impl(&self, id: &str) -> bool {
        self.texture_cache.contains_key(id)
    }

    pub fn trim_memory_level(&mut self, level: MemoryTrimLevel) {
        unsafe {
            match level {
                MemoryTrimLevel::Normal => {
                    let current_frame = self.texture_cache.current_frame;
                    while self.texture_cache.total_vram_bytes > self.texture_cache.max_vram_bytes
                        && self.texture_cache.len() > 1
                    {
                        if let Some(key) =
                            self.texture_cache.pop_lru_candidate(true, current_frame)
                        {
                            if let Some(removed) = self.texture_cache.remove(&key) {
                                self.gl.delete_texture(removed.texture);
                                self.texture_cache.total_vram_bytes = self
                                    .texture_cache
                                    .total_vram_bytes
                                    .saturating_sub(removed.size_bytes);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                MemoryTrimLevel::Moderate => {
                    let current_frame = self.texture_cache.current_frame;
                    let target = self.texture_cache.max_vram_bytes / 2;
                    while self.texture_cache.total_vram_bytes > target
                        && self.texture_cache.len() > 1
                    {
                        if let Some(key) =
                            self.texture_cache.pop_lru_candidate(true, current_frame)
                        {
                            if let Some(removed) = self.texture_cache.remove(&key) {
                                self.gl.delete_texture(removed.texture);
                                self.texture_cache.total_vram_bytes = self
                                    .texture_cache
                                    .total_vram_bytes
                                    .saturating_sub(removed.size_bytes);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                MemoryTrimLevel::Critical => {
                    let mut keys_to_delete = Vec::new();
                    let current_frame = self.texture_cache.current_frame;
                    while let Some(key) =
                        self.texture_cache.pop_lru_candidate(true, current_frame + 1)
                    {
                        if key.as_str() == "__system_wallpaper__" {
                            break;
                        }
                        keys_to_delete.push(key);
                    }
                    for key in keys_to_delete {
                        if let Some(removed) = self.texture_cache.remove(&key) {
                            self.gl.delete_texture(removed.texture);
                            self.texture_cache.total_vram_bytes = self
                                .texture_cache
                                .total_vram_bytes
                                .saturating_sub(removed.size_bytes);
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn load_encrypted_image_in_place(
        &mut self,
        id: &str,
        encrypted_bytes: &mut [u8],
        key: u8,
        width: u32,
        height: u32,
    ) {
        for b in encrypted_bytes.iter_mut() {
            *b ^= key;
        }
        self.load_image_impl(id, encrypted_bytes, width, height);
    }

    pub(crate) fn load_wallpaper_impl(&mut self, rgba_pixels: &[u8], width: u32, height: u32) {
        unsafe {
            if let Some(removed) = self.texture_cache.remove("__system_wallpaper__") {
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
                sniffer_core::style::ObjectFit::Cover,
            );
            self.draw_rect(full_rect, 0x4000_0000, 0.0, 0.0, None);
        }
    }

    pub(crate) fn draw_image_impl(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: sniffer_core::style::ObjectFit,
    ) {
        let current_frame = self.texture_cache.current_frame;

        if let Some(handle) = self.texture_cache.get_mut(id) {
            handle.last_frame = current_frame;
            let tex = handle.texture;
            let img_w = handle.width;
            let img_h = handle.height;

            let mut draw_rect = rect;
            let mut uv_scale = (1.0_f32, 1.0_f32);
            let mut uv_offset = (0.0_f32, 0.0_f32);

            let img_aspect = img_w / img_h;
            let rect_aspect = rect.width / rect.height;

            match object_fit {
                sniffer_core::style::ObjectFit::Fill => {}
                sniffer_core::style::ObjectFit::Contain => {
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
                sniffer_core::style::ObjectFit::Cover => {
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

                let u = self.image_uniforms.clone();

                self.gl.uniform_2_f32(
                    u.u_resolution.as_ref(),
                    self.resolution.0,
                    self.resolution.1,
                );
                self.gl
                    .uniform_2_f32(u.u_rect_pos.as_ref(), draw_rect.x, draw_rect.y);
                self.gl
                    .uniform_2_f32(u.u_rect_size.as_ref(), draw_rect.width, draw_rect.height);
                self.gl
                    .uniform_2_f32(u.u_uv_scale.as_ref(), uv_scale.0, uv_scale.1);
                self.gl
                    .uniform_2_f32(u.u_uv_offset.as_ref(), uv_offset.0, uv_offset.1);
                self.gl.uniform_1_f32(u.u_radius.as_ref(), radius);
                self.gl
                    .uniform_1_f32(u.u_global_alpha.as_ref(), self.global_alpha);

                let t = self.transform_stack.last().unwrap();
                self.gl
                    .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
                self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }
        }
    }
}

