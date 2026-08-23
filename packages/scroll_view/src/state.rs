use sniffer_pkg::error::PackageError;

/// Mutable physics and configuration state for a single scroll view.
#[derive(Clone, Debug)]
pub struct ScrollState {
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub max_scroll_x: f32,
    pub max_scroll_y: f32,
    pub snap_x: Option<f32>,
    pub snap_y: Option<f32>,
    pub page_count: u32,
    pub current_page: i32,
    pub rubber_band: f32,
    pub momentum_damping: f32,
    pub on_snap_callback: Option<String>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            scroll_x: 0.0,
            scroll_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            max_scroll_x: 0.0,
            max_scroll_y: 0.0,
            snap_x: None,
            snap_y: None,
            page_count: 0,
            current_page: 0,
            rubber_band: 0.3,
            momentum_damping: 0.015,
            on_snap_callback: None,
        }
    }
}

impl ScrollState {
    pub fn apply_descriptor(&mut self, descriptor_json: &str) -> Result<(), PackageError> {
        let v: serde_json::Value = serde_json::from_str(descriptor_json)
            .map_err(|e| PackageError::InvalidDescriptor(e.to_string()))?;

        if let Some(sx) = v.get("snap_x") {
            self.snap_x = sx.as_f64().map(|f| f as f32).filter(|&f| f > 0.0);
        }
        if let Some(sy) = v.get("snap_y") {
            self.snap_y = sy.as_f64().map(|f| f as f32).filter(|&f| f > 0.0);
        }
        if let Some(rb) = v.get("rubber_band") {
            if let Some(f) = rb.as_f64() {
                self.rubber_band = (f as f32).clamp(0.0, 1.0);
            }
        }
        if let Some(pc) = v.get("page_count") {
            if let Some(n) = pc.as_u64() {
                self.page_count = n as u32;
            }
        }
        if let Some(mx) = v.get("max_scroll_x") {
            if let Some(f) = mx.as_f64() {
                self.max_scroll_x = (f as f32).max(0.0);
            }
        }
        if let Some(my) = v.get("max_scroll_y") {
            if let Some(f) = my.as_f64() {
                self.max_scroll_y = (f as f32).max(0.0);
            }
        }
        if let Some(md) = v.get("momentum_damping") {
            if let Some(f) = md.as_f64() {
                self.momentum_damping = (f as f32).clamp(0.0, 1.0);
            }
        }
        if let Some(cb) = v.get("on_snap") {
            self.on_snap_callback = cb.as_str().map(ToString::to_string);
        }

        Ok(())
    }
}
