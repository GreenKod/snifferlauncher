use glow::HasContext;
use std::collections::HashMap;

/// Represents the allocated region for a sub-image within the texture atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Normalized UV coordinates: `[u_min, v_min, u_max, v_max]`
    pub uv_rect: [f32; 4],
}

/// Dynamic GPU Texture Atlas for packing icons and small images (`<= 256x256`).
///
/// Consolidates hundreds of separate UI icons into a single GPU texture
/// (e.g. 2048x2048 RGBA = 16 MB), eliminating texture binding overhead
/// and enabling single-call instanced icon drawing (`glDrawArraysInstanced`).
pub struct IconAtlas {
    pub texture: glow::Texture,
    pub width: u32,
    pub height: u32,
    pub current_x: u32,
    pub current_y: u32,
    pub shelf_height: u32,
    pub padding: u32,
    pub regions: HashMap<String, AtlasRegion>,
    pub total_vram_bytes: usize,
}

impl IconAtlas {
    pub const DEFAULT_ATLAS_DIM: u32 = 2048;
    pub const MAX_ICON_DIM: u32 = 256;
    pub const DEFAULT_PADDING: u32 = 1;

    /// Creates a new GPU Texture Atlas with transparent black backing.
    ///
    /// # Errors
    ///
    /// Returns an error if OpenGL texture creation or initialization fails.
    #[allow(clippy::cast_possible_wrap)]
    pub unsafe fn new(gl: &glow::Context, width: u32, height: u32) -> Result<Self, String> {
        unsafe {
            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            let total_vram_bytes = (width as usize) * (height as usize) * 4;
            let zeroes = vec![0u8; total_vram_bytes];

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA).map_err(|e| e.to_string())?,
                i32::try_from(width).map_err(|e| e.to_string())?,
                i32::try_from(height).map_err(|e| e.to_string())?,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&zeroes)),
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                i32::try_from(glow::LINEAR).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                i32::try_from(glow::LINEAR).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                i32::try_from(glow::CLAMP_TO_EDGE).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                i32::try_from(glow::CLAMP_TO_EDGE).map_err(|e| e.to_string())?,
            );

            Ok(Self::new_with_texture(texture, width, height))
        }
    }

    /// Creates an `IconAtlas` instance wrapping an existing `glow::Texture`.
    /// Useful for testing, mocks, or externally allocated OpenGL textures.
    #[must_use]
    pub fn new_with_texture(texture: glow::Texture, width: u32, height: u32) -> Self {
        let total_vram_bytes = (width as usize) * (height as usize) * 4;
        Self {
            texture,
            width,
            height,
            current_x: 0,
            current_y: 0,
            shelf_height: 0,
            padding: Self::DEFAULT_PADDING,
            regions: HashMap::new(),
            total_vram_bytes,
        }
    }

    /// Checks whether an image can fit in the icon atlas based on dimensions.
    #[inline]
    #[must_use]
    pub fn can_pack(&self, width: u32, height: u32) -> bool {
        width > 0 && height > 0 && width <= Self::MAX_ICON_DIM && height <= Self::MAX_ICON_DIM
    }

    /// Allocates shelf space for an image without uploading to GPU.
    ///
    /// Computes non-bleeding normalized UV coordinates `[u_min, v_min, u_max, v_max]`
    /// that strictly enclose the image pixels while maintaining 1px boundary padding.
    pub fn allocate_slot(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        let alloc_w = width.saturating_add(self.padding);
        let alloc_h = height.saturating_add(self.padding);

        if alloc_w > self.width || alloc_h > self.height {
            return None;
        }

        // Check if fits on current shelf
        if self.current_x.saturating_add(alloc_w) > self.width {
            let next_y = self.current_y.saturating_add(self.shelf_height);
            if next_y.saturating_add(alloc_h) > self.height {
                // Atlas is full
                return None;
            }
            self.current_y = next_y;
            self.current_x = 0;
            self.shelf_height = 0;
        }

        let x = self.current_x;
        let y = self.current_y;

        self.current_x = self.current_x.saturating_add(alloc_w);
        self.shelf_height = self.shelf_height.max(alloc_h);

        #[allow(clippy::cast_precision_loss)]
        let u_min = (x as f32) / (self.width as f32);
        #[allow(clippy::cast_precision_loss)]
        let v_min = (y as f32) / (self.height as f32);
        #[allow(clippy::cast_precision_loss)]
        let u_max = ((x + width) as f32) / (self.width as f32);
        #[allow(clippy::cast_precision_loss)]
        let v_max = ((y + height) as f32) / (self.height as f32);

        Some(AtlasRegion {
            x,
            y,
            width,
            height,
            uv_rect: [u_min, v_min, u_max, v_max],
        })
    }

    /// Uploads an RGBA sub-image into the atlas and records its region.
    /// If the id already exists with identical dimensions, the existing region is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are invalid, the atlas is full, or GL upload fails.
    #[allow(clippy::cast_possible_wrap)]
    pub unsafe fn upload(
        &mut self,
        gl: &glow::Context,
        id: &str,
        rgba_pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<AtlasRegion, String> {
        if !self.can_pack(width, height) {
            return Err(format!(
                "Image size ({width}x{height}) exceeds max atlas icon size ({})",
                Self::MAX_ICON_DIM
            ));
        }

        if let Some(existing) = self.regions.get(id) {
            if existing.width == width && existing.height == height {
                return Ok(*existing);
            }
        }

        let region = self.allocate_slot(width, height).ok_or_else(|| {
            format!(
                "IconAtlas out of space ({}/{} px)",
                self.current_x, self.current_y
            )
        })?;

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(region.x).map_err(|e| e.to_string())?,
                i32::try_from(region.y).map_err(|e| e.to_string())?,
                i32::try_from(region.width).map_err(|e| e.to_string())?,
                i32::try_from(region.height).map_err(|e| e.to_string())?,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba_pixels)),
            );
        }

        self.regions.insert(id.to_string(), region);
        Ok(region)
    }

    #[inline]
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&AtlasRegion> {
        self.regions.get(id)
    }

    #[inline]
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.regions.contains_key(id)
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Resets atlas layout and clears regions without destroying the GPU texture.
    pub fn clear(&mut self) {
        self.regions.clear();
        self.current_x = 0;
        self.current_y = 0;
        self.shelf_height = 0;
    }

    /// Destroys the underlying OpenGL texture object.
    pub unsafe fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
        }
        self.clear();
    }

    #[inline]
    #[must_use]
    pub fn vram_bytes(&self) -> usize {
        self.total_vram_bytes
    }
}
