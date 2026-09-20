use crate::math::Rect;

/// Represents a 2D clipping region with local rectangle bounds, corner radius, and transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRegion {
    pub rect: Rect,
    pub radius: f32,
    pub transform: [f32; 9],
}

impl ClipRegion {
    pub const IDENTITY_TRANSFORM: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

    #[must_use]
    pub fn new(rect: Rect, radius: f32, transform: [f32; 9]) -> Self {
        Self {
            rect,
            radius: radius.max(0.0),
            transform,
        }
    }

    #[must_use]
    pub fn from_rect(rect: Rect) -> Self {
        Self::new(rect, 0.0, Self::IDENTITY_TRANSFORM)
    }

    #[must_use]
    pub fn is_rounded(&self) -> bool {
        self.radius > 0.0
    }

    /// Check if the transformation matrix is axis-aligned (i.e. no rotation or shear).
    #[must_use]
    pub fn is_axis_aligned(&self) -> bool {
        self.transform[1].abs() < 1e-5 && self.transform[3].abs() < 1e-5
    }

    /// Transforms a point (x, y) from local coordinates to screen coordinates.
    #[must_use]
    pub fn transform_point(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let t = self.transform;
        let new_x = t[0].mul_add(x, t[3].mul_add(y, t[6]));
        let new_y = t[1].mul_add(x, t[4].mul_add(y, t[7]));
        (new_x, new_y)
    }

    /// Computes the axis-aligned bounding box of this clip region in screen coordinates.
    #[must_use]
    pub fn screen_bounds(&self) -> Rect {
        let (x1, y1) = self.transform_point((self.rect.x, self.rect.y));
        let (x2, y2) = self.transform_point((self.rect.x + self.rect.width, self.rect.y));
        let (x3, y3) = self.transform_point((self.rect.x, self.rect.y + self.rect.height));
        let (x4, y4) = self.transform_point((
            self.rect.x + self.rect.width,
            self.rect.y + self.rect.height,
        ));

        let min_x = x1.min(x2).min(x3).min(x4);
        let max_x = x1.max(x2).max(x3).max(x4);
        let min_y = y1.min(y2).min(y3).min(y4);
        let max_y = y1.max(y2).max(y3).max(y4);

        Rect::new(
            min_x,
            min_y,
            (max_x - min_x).max(0.0),
            (max_y - min_y).max(0.0),
        )
    }

    /// Computes the 3x3 inverse transformation matrix (column-major).
    /// Used by fragment shaders to map screen pixel positions back to the local clip coordinate space.
    #[must_use]
    pub fn inverse_transform(&self) -> [f32; 9] {
        let [a, b, _, c, d, _, tx, ty, _] = self.transform;
        let det = a.mul_add(d, -(b * c));
        if det.abs() < 1e-6 {
            return Self::IDENTITY_TRANSFORM;
        }
        let inv_det = 1.0 / det;
        let inv_a = d * inv_det;
        let inv_b = -b * inv_det;
        let inv_c = -c * inv_det;
        let inv_d = a * inv_det;
        let inv_tx = -(inv_a.mul_add(tx, inv_c * ty));
        let inv_ty = -(inv_b.mul_add(tx, inv_d * ty));

        [inv_a, inv_b, 0.0, inv_c, inv_d, 0.0, inv_tx, inv_ty, 1.0]
    }

    /// Calculates the intersection between this region's screen bounds and another screen rectangle.
    #[must_use]
    pub fn intersect_screen_bounds(&self, other_bounds: Rect) -> Rect {
        let my_bounds = self.screen_bounds();
        let cx = my_bounds.x.max(other_bounds.x);
        let cy = my_bounds.y.max(other_bounds.y);
        let cw = (my_bounds.x + my_bounds.width).min(other_bounds.x + other_bounds.width) - cx;
        let ch = (my_bounds.y + my_bounds.height).min(other_bounds.y + other_bounds.height) - cy;
        Rect::new(cx, cy, cw.max(0.0), ch.max(0.0))
    }
}

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
    fn draw_elevation_shadow(
        &mut self,
        rect: Rect,
        radius: f32,
        elevation: f32,
        shadow_color: Option<u32>,
    ) {
        if elevation <= 0.0 {
            return;
        }
        let color = shadow_color.unwrap_or(0x4D00_0000);
        self.draw_shadow(rect, radius, elevation * 0.75, elevation * 0.3, color);
    }
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32);
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32);
    fn begin_frame(&mut self, width: f32, height: f32);
    fn end_frame(&mut self);
    fn flush(&mut self) {}
    fn set_clip_rect(&mut self, rect: Rect);
    fn clear_clip_rect(&mut self);
    fn push_clip_rect(&mut self, rect: Rect, radius: f32) {
        self.push_clip_region(ClipRegion::new(
            rect,
            radius,
            ClipRegion::IDENTITY_TRANSFORM,
        ));
    }
    fn push_clip_region(&mut self, region: ClipRegion) {
        self.set_clip_rect(region.rect);
    }
    fn pop_clip_rect(&mut self) {
        self.clear_clip_rect();
    }
    fn current_clip(&self) -> Option<&ClipRegion> {
        None
    }
    fn push_transform(
        &mut self,
        _cx: f32,
        _cy: f32,
        _scale: f32,
        _rotate: f32,
        _tx: f32,
        _ty: f32,
    ) {
    }
    fn pop_transform(&mut self) {}
    fn set_global_alpha(&mut self, alpha: f32);
    fn load_image(&mut self, id: &str, rgba_pixels: &[u8], width: u32, height: u32);
    fn has_image(&self, id: &str) -> bool;
    fn draw_image(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: crate::style::ObjectFit,
    );
    fn load_wallpaper(&mut self, _rgba_pixels: &[u8], _width: u32, _height: u32) {}
    fn draw_wallpaper(&mut self, _width: f32, _height: f32) {}
    fn draw_backdrop_blur(
        &mut self,
        _rect: Rect,
        _radius: f32,
        _blur_radius: f32,
        _tint: Option<u32>,
    ) {
    }
    fn measure_text(&self, text: &str, size: f32) -> f32;
    /// Returns the ascent (distance from the top of the text box to the baseline)
    /// at the given size. Used to correctly center text vertically within a rect.
    fn text_ascent(&self, size: f32) -> f32 {
        // Default: assume ascent is 75% of size (typical for most fonts)
        size * 0.75
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_region_creation_and_properties() {
        let rect = Rect::new(10.0, 20.0, 100.0, 200.0);
        let sharp = ClipRegion::from_rect(rect);
        assert!(!sharp.is_rounded());
        assert!(sharp.is_axis_aligned());
        assert_eq!(sharp.rect, rect);
        assert_eq!(sharp.radius, 0.0);

        let rounded = ClipRegion::new(rect, 16.0, ClipRegion::IDENTITY_TRANSFORM);
        assert!(rounded.is_rounded());
        assert!(rounded.is_axis_aligned());
        assert_eq!(rounded.radius, 16.0);
    }

    #[test]
    fn test_clip_region_screen_bounds_axis_aligned() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let region = ClipRegion::from_rect(rect);
        let bounds = region.screen_bounds();
        assert_eq!(bounds, rect);
    }

    #[test]
    fn test_clip_region_translation() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        // Translate by tx=30, ty=40
        let transform = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 30.0, 40.0, 1.0];
        let region = ClipRegion::new(rect, 8.0, transform);
        assert!(region.is_axis_aligned());

        let bounds = region.screen_bounds();
        assert_eq!(bounds.x, 40.0);
        assert_eq!(bounds.y, 60.0);
        assert_eq!(bounds.width, 100.0);
        assert_eq!(bounds.height, 50.0);

        let inv = region.inverse_transform();
        // inv should translate by -30, -40
        assert_eq!(inv[6], -30.0);
        assert_eq!(inv[7], -40.0);
    }

    #[test]
    fn test_clip_region_nested_intersection() {
        let parent = ClipRegion::from_rect(Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = ClipRegion::from_rect(Rect::new(50.0, 50.0, 300.0, 100.0));

        let intersected = child.intersect_screen_bounds(parent.screen_bounds());
        assert_eq!(intersected.x, 50.0);
        assert_eq!(intersected.y, 50.0);
        assert_eq!(intersected.width, 150.0);
        assert_eq!(intersected.height, 100.0);
    }
}
