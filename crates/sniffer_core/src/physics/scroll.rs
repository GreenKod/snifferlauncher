use crate::anim::{SpringConfig, SpringSimulation};

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

    pub spring_sim_x: Option<SpringSimulation>,
    pub spring_sim_y: Option<SpringSimulation>,

    pub last_snap_page: i32,
    pub snap_just_completed: bool,

    pub snap_x: Option<f32>,
    pub max_y: Option<f32>,
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
            spring_sim_x: None,
            spring_sim_y: None,
            last_snap_page: 0,
            snap_just_completed: false,
            snap_x: None,
            max_y: None,
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
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.is_dragging
            || self.snap_target_x.is_some()
            || self.spring_sim_x.as_ref().is_some_and(|s| !s.is_at_rest())
            || self.spring_sim_y.as_ref().is_some_and(|s| !s.is_at_rest())
            || self.vel_x.abs() > 0.5
            || self.vel_y.abs() > 0.5
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = false;
        self.snap_just_completed = false;

        let max_limit = self
            .snap_x
            .zip(self.page_count)
            .map_or(f32::MAX, |(s, n)| s * (n as f32 - 1.0));

        // 1. Horizontal Motion: Spring Snapping & Rubber-Band Recovery
        if let Some(ref mut sim) = self.spring_sim_x {
            self.pos_x = sim.step(dt);
            self.vel_x = sim.velocity();
            active = true;

            if sim.is_at_rest() {
                self.pos_x = sim.target();
                self.vel_x = 0.0;
                let was_snap = self.snap_target_x.is_some();
                self.snap_target_x = None;
                self.spring_sim_x = None;
                self.spring_time = 0.0;
                if was_snap {
                    self.snap_just_completed = true;
                }
                active = false;
            }
        } else if let Some(target) = self.snap_target_x {
            let mut sim = SpringSimulation::new(SpringConfig::snappy(), self.pos_x, target)
                .with_initial_velocity(self.vel_x);
            self.pos_x = sim.step(dt);
            self.vel_x = sim.velocity();
            active = true;
            if sim.is_at_rest() {
                self.pos_x = target;
                self.vel_x = 0.0;
                self.snap_target_x = None;
                self.spring_sim_x = None;
                self.snap_just_completed = true;
                active = false;
            } else {
                self.spring_sim_x = Some(sim);
            }
        } else if self.vel_x.abs() > 0.5 {
            let lambda = 7.62_f32;
            let decay = (-lambda * dt).exp();
            self.pos_x += self.vel_x * dt;
            self.vel_x *= decay;
            active = true;

            if self.pos_x < 0.0 {
                if !self.is_dragging {
                    self.spring_sim_x = Some(
                        SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, 0.0)
                            .with_initial_velocity(self.vel_x),
                    );
                } else {
                    self.pos_x = 0.0;
                    self.vel_x = 0.0;
                }
            } else if max_limit < f32::MAX && self.pos_x > max_limit {
                if !self.is_dragging {
                    self.spring_sim_x = Some(
                        SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, max_limit)
                            .with_initial_velocity(self.vel_x),
                    );
                } else {
                    self.pos_x = max_limit;
                    self.vel_x = 0.0;
                }
            }
        } else if !self.is_dragging {
            if self.pos_x < -0.1 {
                self.spring_sim_x = Some(
                    SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, 0.0)
                        .with_initial_velocity(self.vel_x),
                );
                active = true;
            } else if max_limit < f32::MAX && self.pos_x > max_limit + 0.1 {
                self.spring_sim_x = Some(
                    SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, max_limit)
                        .with_initial_velocity(self.vel_x),
                );
                active = true;
            }
        }

        // 2. Vertical Motion: Rubber-Band Springs & Kinetic Decay
        let max_limit_y = self.max_y.unwrap_or(0.0);

        if let Some(ref mut sim) = self.spring_sim_y {
            self.pos_y = sim.step(dt);
            self.vel_y = sim.velocity();
            active = true;

            if sim.is_at_rest() {
                self.pos_y = sim.target();
                self.vel_y = 0.0;
                self.spring_sim_y = None;
            }
        } else if self.vel_y.abs() > 0.5 {
            let lambda = 7.62_f32;
            let decay = (-lambda * dt).exp();
            self.pos_y += self.vel_y * dt;
            self.vel_y *= decay;
            active = true;

            if self.pos_y < 0.0 {
                if !self.is_dragging {
                    self.spring_sim_y = Some(
                        SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, 0.0)
                            .with_initial_velocity(self.vel_y),
                    );
                } else {
                    self.pos_y = 0.0;
                    self.vel_y = 0.0;
                }
            } else if self.pos_y > max_limit_y {
                if !self.is_dragging {
                    self.spring_sim_y = Some(
                        SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, max_limit_y)
                            .with_initial_velocity(self.vel_y),
                    );
                } else {
                    self.pos_y = max_limit_y;
                    self.vel_y = 0.0;
                }
            }
        } else if !self.is_dragging {
            if self.pos_y < -0.1 {
                self.spring_sim_y = Some(
                    SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, 0.0)
                        .with_initial_velocity(self.vel_y),
                );
                active = true;
            } else if self.pos_y > max_limit_y + 0.1 {
                self.spring_sim_y = Some(
                    SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, max_limit_y)
                        .with_initial_velocity(self.vel_y),
                );
                active = true;
            }
        }

        active || self.snap_just_completed
    }

    pub fn apply_drag(&mut self, delta_x: f32) {
        self.snap_target_x = None;
        self.spring_sim_x = None;

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

    pub fn apply_drag_y(&mut self, delta_y: f32) {
        self.spring_sim_y = None;
        let max_limit_y = self.max_y.unwrap_or(0.0);
        let coeff = self.rubber_band.unwrap_or(RUBBER_BAND_COEFF);
        let new_pos = self.pos_y + delta_y;
        if new_pos < 0.0 {
            self.pos_y = new_pos * coeff;
        } else if new_pos > max_limit_y {
            self.pos_y = max_limit_y + (new_pos - max_limit_y) * coeff;
        } else {
            self.pos_y = new_pos;
        }
    }

    pub fn release_drag_y(&mut self, vel_y: f32) {
        self.is_dragging = false;
        let max_limit_y = self.max_y.unwrap_or(0.0);
        if self.pos_y < 0.0 {
            self.vel_y = 0.0;
            self.spring_sim_y = Some(
                SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, 0.0)
                    .with_initial_velocity(vel_y),
            );
        } else if self.pos_y > max_limit_y {
            self.vel_y = 0.0;
            self.spring_sim_y = Some(
                SpringSimulation::new(SpringConfig::rubber_band(), self.pos_y, max_limit_y)
                    .with_initial_velocity(vel_y),
            );
        } else {
            self.vel_y = vel_y;
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

                // Analytical spring simulation for page snap
                self.spring_sim_x = Some(
                    SpringSimulation::new(SpringConfig::snappy(), self.pos_x, target_x)
                        .with_initial_velocity(init_vel),
                );
            }
        } else if self.pos_x < 0.0 {
            self.spring_sim_x = Some(
                SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, 0.0)
                    .with_initial_velocity(vel_x),
            );
        } else if max_x < f32::MAX && self.pos_x > max_x {
            self.spring_sim_x = Some(
                SpringSimulation::new(SpringConfig::rubber_band(), self.pos_x, max_x)
                    .with_initial_velocity(vel_x),
            );
        }
    }
}
