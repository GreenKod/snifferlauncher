#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// DPI-aware screen information used to make the layout responsive.
///
/// All layout values are expressed in **physical pixels** (as OpenGL expects).
/// The `scale_factor` converts logical dp units to physical pixels:
///   `physical_px = logical_dp * scale_factor`
///
/// Baseline: 160 DPI → `scale_factor` = 1.0 (matches Android mdpi).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenMetrics {
    /// Physical pixel width of the rendering surface.
    pub physical_width: f32,
    /// Physical pixel height of the rendering surface.
    pub physical_height: f32,
    /// Pixels per dp (logical density unit). 160 dpi = 1.0.
    pub scale_factor: f32,
    /// Font scaling factor (Android scaledDensity). Accounts for user font size preference.
    pub font_scale_factor: f32,
}

impl ScreenMetrics {
    /// Construct `ScreenMetrics` from a known DPI value.
    /// `dpi = 160` → `scale_factor` 1.0 (Android mdpi baseline).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // f32::clamp is not const
    pub fn from_dpi(physical_width: f32, physical_height: f32, dpi: f32) -> Self {
        let scale_factor = (dpi / 160.0).clamp(0.75, 4.0);
        Self {
            physical_width,
            physical_height,
            scale_factor,
            font_scale_factor: scale_factor,
        }
    }

    /// Construct with explicit scale factors (e.g. from Android `DisplayMetrics`).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // f32::clamp is not const
    pub fn from_scale(
        physical_width: f32,
        physical_height: f32,
        scale_factor: f32,
        font_scale_factor: f32,
    ) -> Self {
        Self {
            physical_width,
            physical_height,
            scale_factor: scale_factor.clamp(0.75, 4.0),
            font_scale_factor: font_scale_factor.clamp(0.75, 6.0),
        }
    }

    /// Fallback: assume 160 DPI (1× density) — safe default when DPI is unavailable.
    #[must_use]
    pub const fn default_mdpi(physical_width: f32, physical_height: f32) -> Self {
        Self {
            physical_width,
            physical_height,
            scale_factor: 1.0,
            font_scale_factor: 1.0,
        }
    }

    /// Logical width in dp units (`physical_width / scale_factor`).
    #[must_use]
    pub fn logical_width(&self) -> f32 {
        self.physical_width / self.scale_factor
    }

    /// Logical height in dp units.
    #[must_use]
    pub fn logical_height(&self) -> f32 {
        self.physical_height / self.scale_factor
    }

    /// Scale a logical dp value to physical pixels.
    #[must_use]
    pub fn dp(&self, value: f32) -> f32 {
        value * self.scale_factor
    }

    /// Scale a logical sp (font) value to physical pixels using the `font_scale_factor`.
    #[must_use]
    pub fn sp(&self, value: f32) -> f32 {
        value * self.font_scale_factor
    }

    /// Returns `true` when the logical screen width is below 320 dp (very compact/small phone).
    #[must_use]
    pub fn is_very_compact(&self) -> bool {
        self.logical_width() < 320.0
    }

    /// Returns `true` when the logical screen width is below 360 dp (compact/small phone).
    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.logical_width() < 360.0
    }

    /// Returns `true` when the logical screen width is ≥ 600 dp (tablet/large display).
    #[must_use]
    pub fn is_large(&self) -> bool {
        self.logical_width() >= 600.0
    }
}

