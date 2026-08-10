use crate::core::style::{Easing, Style};
use std::collections::HashMap;

/// Interpolates between two f32 values based on t (0.0 to 1.0).
#[must_use]
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    let t_safe = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    let val = (end - start).mul_add(t_safe, start);
    if val.is_nan() || val.is_infinite() {
        end
    } else {
        val
    }
}

/// Interpolates between two ARGB colors.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn lerp_color(start: u32, end: u32, t: f32) -> u32 {
    let sa = ((start >> 24) & 0xff) as f32;
    let sr = ((start >> 16) & 0xff) as f32;
    let sg = ((start >> 8) & 0xff) as f32;
    let sb = (start & 0xff) as f32;

    let ea = ((end >> 24) & 0xff) as f32;
    let er = ((end >> 16) & 0xff) as f32;
    let eg = ((end >> 8) & 0xff) as f32;
    let eb = (end & 0xff) as f32;

    let alpha = lerp(sa, ea, t) as u32;
    let red = lerp(sr, er, t) as u32;
    let green = lerp(sg, eg, t) as u32;
    let blue = lerp(sb, eb, t) as u32;

    (alpha << 24) | (red << 16) | (green << 8) | blue
}

/// Interpolates between two Option<u32> colors (fade in/out support).
#[must_use]
pub fn lerp_opt_color(start: Option<u32>, end: Option<u32>, t: f32) -> Option<u32> {
    match (start, end) {
        (Some(s), Some(e)) => Some(lerp_color(s, e, t)),
        (Some(s), None) => {
            let transparent = s & 0x00FF_FFFF; // keep rgb, alpha 0
            Some(lerp_color(s, transparent, t))
        }
        (None, Some(e)) => {
            let transparent = e & 0x00FF_FFFF; // keep rgb, alpha 0
            Some(lerp_color(transparent, e, t))
        }
        (None, None) => None,
    }
}

#[must_use]
pub fn evaluate_easing(easing: &Easing, mut t: f32) -> f32 {
    t = t.clamp(0.0, 1.0);

    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                2.0f32.mul_add(-t, 4.0).mul_add(t, -1.0)
            }
        }
        Easing::Spring { stiffness, damping } => {
            let constant_four = (2.0 * std::f32::consts::PI) / (*damping * 3.0).max(1.0);
            if t == 0.0 || (t - 1.0).abs() < f32::EPSILON {
                t
            } else {
                let power = (-10.0 * t * (*stiffness / 100.0).max(1.0)).exp2();
                power * (t.mul_add(10.0, -0.75) * constant_four).sin() + 1.0
            }
        }
    }
}

#[derive(Clone)]
pub struct AnimState {
    pub start_style: Style,
    pub target_style: Style,
    pub current_style: Style,
    pub time_elapsed: f32,
    pub is_active: bool,
}

impl AnimState {
    pub fn update(&mut self, dt: f32) {
        if !self.is_active {
            return;
        }

        self.time_elapsed += dt;
        let duration = self.target_style.transition.duration;

        let mut t = if duration > 0.0 {
            self.time_elapsed / duration
        } else {
            1.0
        };

        if t >= 1.0 {
            t = 1.0;
            self.is_active = false;
        }

        let eased_t = evaluate_easing(&self.target_style.transition.easing, t);

        // Copy everything from target_style so non-animated properties (e.g. text_size, layout) update instantly
        self.current_style = self.target_style.clone();

        // Interpolate properties
        self.current_style.opacity =
            lerp(self.start_style.opacity, self.target_style.opacity, eased_t);
        self.current_style.background_color = lerp_opt_color(
            self.start_style.background_color,
            self.target_style.background_color,
            eased_t,
        );
        self.current_style.text_color = lerp_opt_color(
            self.start_style.text_color,
            self.target_style.text_color,
            eased_t,
        );
        self.current_style.border_color = lerp_opt_color(
            self.start_style.border_color,
            self.target_style.border_color,
            eased_t,
        );
        self.current_style.border_radius = lerp(
            self.start_style.border_radius,
            self.target_style.border_radius,
            eased_t,
        );

        // Transforms
        self.current_style.transform.scale = lerp(
            self.start_style.transform.scale,
            self.target_style.transform.scale,
            eased_t,
        );
        self.current_style.transform.translate_x = lerp(
            self.start_style.transform.translate_x,
            self.target_style.transform.translate_x,
            eased_t,
        );
        self.current_style.transform.translate_y = lerp(
            self.start_style.transform.translate_y,
            self.target_style.transform.translate_y,
            eased_t,
        );
        self.current_style.transform.rotate = lerp(
            self.start_style.transform.rotate,
            self.target_style.transform.rotate,
            eased_t,
        );

        // Note: Layout dimensions (width, height, etc) are not interpolated here in v1
        // because changing them requires full Taffy re-layout. For performance, we stick to
        // transforms for resizing/moving during animations.
    }
}

#[derive(Default, Clone)]
pub struct TransitionManager {
    pub states: HashMap<String, AnimState>,
}

impl TransitionManager {
    /// Syncs the manager with a new target style for a given element ID.
    pub fn update_target(&mut self, id: &str, new_style: &Style) {
        if let Some(state) = self.states.get_mut(id) {
            // Only trigger transition if properties we care about actually changed.
            if state.target_style != *new_style {
                state.start_style = state.current_style.clone();
                state.target_style = new_style.clone();
                state.time_elapsed = 0.0;
                if new_style.transition.duration > 0.0 {
                    state.is_active = true;
                } else {
                    // Instant apply
                    state.current_style = new_style.clone();
                    state.is_active = false;
                }
            }
        } else {
            let s = new_style.clone();
            self.states.insert(
                id.to_string(),
                AnimState {
                    start_style: s.clone(),
                    target_style: s.clone(),
                    current_style: s,
                    time_elapsed: 0.0,
                    is_active: false,
                },
            );
        }
    }

    /// Recursively walks the UI element tree and updates the animation target for every element with an ID.
    pub fn sync_tree(&mut self, element: &crate::core::types::Element) {
        if let Some(id) = element.id() {
            self.update_target(id, element.style());
        }
        match element {
            crate::core::types::Element::Container { children, .. }
            | crate::core::types::Element::ScrollView { children, .. } => {
                for child in children {
                    self.sync_tree(child);
                }
            }
            _ => {}
        }
    }

    /// Progresses all active animations by `dt` seconds.
    /// Returns `true` if any animation is active (meaning screen should redraw).
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        for state in self.states.values_mut() {
            if state.is_active {
                state.update(dt);
                needs_redraw = true;
            }
        }
        needs_redraw
    }

    /// Gets the currently interpolated style for drawing.
    #[must_use]
    pub fn get_current_style(&self, id: &str) -> Option<&Style> {
        self.states.get(id).map(|s| &s.current_style)
    }
}
