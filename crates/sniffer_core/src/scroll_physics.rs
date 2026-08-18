use crate::style::{Dimension, FlexDirection};
use crate::types::Element;
use crate::ui::widget::fnv1a;
use std::collections::HashMap;
use std::hash::BuildHasher;

pub const RUBBER_BAND_COEFF: f32 = 0.55;

#[derive(Clone, Debug)]
pub struct ScrollPhysics {
    pub pos_x: f32,
    pub pos_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub is_dragging: bool,

    pub snap_target_x: Option<f32>,
    pub spring_start_x: f32,
    pub spring_start_vel_x: f32,
    pub spring_time: f32,

    pub last_snap_page: i32,
    pub snap_just_completed: bool,

    pub snap_x: Option<f32>,
    pub rubber_band: Option<f32>,
    pub page_count: Option<u32>,
    pub on_snap: Option<String>,
}

impl Default for ScrollPhysics {
    fn default() -> Self {
        Self {
            pos_x: 0.0,
            pos_y: 0.0,
            vel_x: 0.0,
            vel_y: 0.0,
            is_dragging: false,
            snap_target_x: None,
            spring_start_x: 0.0,
            spring_start_vel_x: 0.0,
            spring_time: 0.0,
            last_snap_page: 0,
            snap_just_completed: false,
            snap_x: None,
            rubber_band: None,
            page_count: None,
            on_snap: None,
        }
    }
}

#[must_use]
pub fn rubber_band_clamp(overscroll: f32, dimension: f32, coeff: f32) -> f32 {
    let d = dimension.max(1.0);
    let abs_over = overscroll.abs();
    let damped = d * (1.0 - 1.0 / (coeff * (abs_over / d) + 1.0));
    damped * overscroll.signum()
}

impl ScrollPhysics {
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = false;
        self.snap_just_completed = false;

        let max_limit = self
            .snap_x
            .zip(self.page_count)
            .map_or(f32::MAX, |(s, n)| s * (n as f32 - 1.0));

        if let Some(target) = self.snap_target_x {
            self.spring_time += dt;
            let t = self.spring_time;
            let y0 = self.spring_start_x - target;
            let v0 = self.spring_start_vel_x;

            let omega = 16.0_f32;
            let exp_term = (-omega * t).exp();

            let pos = target + (y0 + (v0 + omega * y0) * t) * exp_term;
            let vel = (v0 - omega * (y0 + (v0 + omega * y0) * t)) * exp_term;

            self.pos_x = pos;
            self.vel_x = vel;
            active = true;

            if exp_term < 0.001 || (self.pos_x - target).abs() < 0.5 {
                self.pos_x = target;
                self.vel_x = 0.0;
                self.snap_target_x = None;
                self.spring_time = 0.0;
                self.snap_just_completed = true;
                active = false;
            }
        } else if self.vel_x.abs() > 0.5 {
            let lambda = 7.62_f32;
            let decay = (-lambda * dt).exp();
            self.pos_x += self.vel_x * dt;
            self.vel_x *= decay;
            active = true;

            if self.pos_x < 0.0 {
                self.pos_x = 0.0;
                self.vel_x = 0.0;
            } else if max_limit < f32::MAX && self.pos_x > max_limit {
                self.pos_x = max_limit;
                self.vel_x = 0.0;
            }
        }

        active || self.snap_just_completed
    }

    pub fn apply_drag(&mut self, delta_x: f32) {
        let max_x = self
            .snap_x
            .zip(self.page_count)
            .map_or(f32::MAX, |(s, n)| s * (n as f32 - 1.0));

        let coeff = self.rubber_band.unwrap_or(RUBBER_BAND_COEFF);

        if self.pos_x < 0.0 || (self.pos_x > max_x && max_x < f32::MAX / 2.0) {
            self.pos_x += delta_x * coeff;
        } else {
            let new_pos = self.pos_x + delta_x;
            if new_pos < 0.0 {
                self.pos_x = new_pos * coeff;
            } else if new_pos > max_x && max_x < f32::MAX / 2.0 {
                self.pos_x = max_x + (new_pos - max_x) * coeff;
            } else {
                self.pos_x = new_pos;
            }
        }
    }

    pub fn release_drag(&mut self, vel_x: f32) {
        self.is_dragging = false;
        self.vel_x = vel_x;

        let max_x = self
            .snap_x
            .zip(self.page_count)
            .map_or(f32::MAX, |(s, n)| s * (n as f32 - 1.0));

        if let Some(page_width) = self.snap_x {
            if page_width > 0.0 {
                let last_page = (self.page_count.unwrap_or(1) as f32) - 1.0;
                let page_float = self.pos_x / page_width;

                let target_page = if vel_x > 80.0 {
                    (page_float.floor() + 1.0).min(last_page)
                } else if vel_x < -80.0 {
                    (page_float.ceil() - 1.0).max(0.0)
                } else {
                    page_float.round().clamp(0.0, last_page)
                };

                let target_x = (target_page * page_width).clamp(0.0, max_x);
                self.snap_target_x = Some(target_x);
                self.spring_start_x = self.pos_x;

                let mut init_vel = self.vel_x;
                if (target_x >= max_x && init_vel > 0.0) || (target_x <= 0.0 && init_vel < 0.0) {
                    init_vel = 0.0;
                }
                self.spring_start_vel_x = init_vel;
                self.spring_time = 0.0;
            }
        }
    }
}

pub fn sync_scroll_physics_from_tree<S: BuildHasher>(
    element: &Element,
    physics: &mut HashMap<u64, ScrollPhysics, S>,
) {
    if let Element::ScrollView {
        id: Some(id_str),
        snap_x,
        snap_y,
        rubber_band,
        page_count,
        on_snap,
        ..
    } = element
    {
        if snap_x.is_some() || snap_y.is_some() {
            let wid = fnv1a(id_str.as_bytes());
            let entry = physics.entry(wid).or_default();
            if let (Some(new_snap_x), Some(old_snap_x)) = (*snap_x, entry.snap_x) {
                if (new_snap_x - old_snap_x).abs() > 0.5 && !entry.is_dragging {
                    let max_page = (entry.page_count.unwrap_or(1) as f32 - 1.0).max(0.0);
                    let clamped_page = (entry.last_snap_page as f32).clamp(0.0, max_page);
                    entry.pos_x = clamped_page * new_snap_x;
                    entry.vel_x = 0.0;
                    entry.vel_y = 0.0;
                    entry.snap_target_x = None;
                }
            }
            entry.snap_x = *snap_x;
            entry.rubber_band = *rubber_band;
            entry.page_count = *page_count;
            entry.on_snap.clone_from(on_snap);
        }
    }

    match element {
        Element::Container { children, .. }
        | Element::ScrollView { children, .. }
        | Element::SharedView { children, .. } => {
            for child in children {
                sync_scroll_physics_from_tree(child, physics);
            }
        }
        _ => {}
    }
}

pub fn inject_physics_to_tree(element: &mut Element, target_id: u64, pos_x: f32, pos_y: f32) {
    if let Element::ScrollView {
        id: Some(id_str),
        scroll_x,
        scroll_y,
        ..
    } = element
    {
        if fnv1a(id_str.as_bytes()) == target_id {
            *scroll_x = pos_x;
            *scroll_y = pos_y;
            return;
        }
    }
    match element {
        Element::Container { children, .. }
        | Element::ScrollView { children, .. }
        | Element::SharedView { children, .. } => {
            for child in children {
                inject_physics_to_tree(child, target_id, pos_x, pos_y);
            }
        }
        _ => {}
    }
}

#[allow(clippy::cast_possible_wrap)]
pub fn update_indicator_dots_in_element(
    element: &mut Element,
    active_page: i32,
    vmin_px: f32,
) -> bool {
    if let Element::Container {
        id,
        children,
        style,
    } = element
    {
        if let Some(id_str) = id {
            if id_str == "page_indicator_container" {
                let is_landscape = style.flex_direction == FlexDirection::Column;
                let mut changed = false;
                for (p, child_el) in children.iter_mut().enumerate() {
                    let is_active = (p as i32) == active_page;
                    if let Element::Container {
                        style: dot_style, ..
                    } = child_el
                    {
                        let new_color = Some(if is_active { 0xFF00_E5FF } else { 0x44FF_FFFF });
                        if dot_style.background_color != new_color {
                            dot_style.background_color = new_color;
                            changed = true;
                        }
                        if is_landscape {
                            let new_h = Dimension::Pixels(if is_active {
                                3.6 * vmin_px
                            } else {
                                1.6 * vmin_px
                            });
                            let new_w = Dimension::Pixels(1.6 * vmin_px);
                            if dot_style.height != new_h || dot_style.width != new_w {
                                dot_style.height = new_h;
                                dot_style.width = new_w;
                                changed = true;
                            }
                        } else {
                            let new_w = Dimension::Pixels(if is_active {
                                3.6 * vmin_px
                            } else {
                                1.6 * vmin_px
                            });
                            let new_h = Dimension::Pixels(1.6 * vmin_px);
                            if dot_style.width != new_w || dot_style.height != new_h {
                                dot_style.width = new_w;
                                dot_style.height = new_h;
                                changed = true;
                            }
                        }
                    }
                }
                return changed;
            }
        }
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    } else if let Element::ScrollView { children, .. } = element {
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    }
    false
}
