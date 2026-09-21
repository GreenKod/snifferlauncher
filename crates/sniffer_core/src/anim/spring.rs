//! Analytical second-order spring physics solver.
//!
//! Solves the continuous differential equation:
//! m * y''(t) + c * y'(t) + k * y(t) = 0
//! exactly in closed form for critically damped, underdamped, and overdamped systems.

/// Configuration parameters defining the spring dynamics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringConfig {
    /// Mass of the object in kilograms (must be > 0.0). Default: 1.0.
    pub mass: f32,
    /// Spring stiffness constant in N/m (must be > 0.0). Default: 180.0.
    pub stiffness: f32,
    /// Damping coefficient in N*s/m (must be >= 0.0). Default: 24.0.
    pub damping: f32,
    /// Position displacement threshold under which the spring is considered settled. Default: 0.05.
    pub position_tolerance: f32,
    /// Velocity threshold under which the spring is considered settled. Default: 0.1.
    pub velocity_tolerance: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            mass: 1.0,
            stiffness: 180.0,
            damping: 24.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        }
    }
}

impl SpringConfig {
    /// Highly responsive and critically damped spring with minimal overshoot.
    #[must_use]
    pub const fn snappy() -> Self {
        Self {
            mass: 1.0,
            stiffness: 300.0,
            damping: 34.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        }
    }

    /// Underdamped bouncy spring for playful tactile responses.
    #[must_use]
    pub const fn bouncy() -> Self {
        Self {
            mass: 1.0,
            stiffness: 220.0,
            damping: 14.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        }
    }

    /// Gentle, smooth spring for page transitions and large layout shifts.
    #[must_use]
    pub const fn gentle() -> Self {
        Self {
            mass: 1.0,
            stiffness: 120.0,
            damping: 20.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        }
    }

    /// Rubber-band return spring for scroll overscroll recovery.
    #[must_use]
    pub const fn rubber_band() -> Self {
        Self {
            mass: 1.0,
            stiffness: 260.0,
            damping: 28.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        }
    }
}

/// Analytical second-order spring physics simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringSimulation {
    /// Spring configuration (mass, stiffness, damping, tolerances).
    pub config: SpringConfig,
    /// Current position of the simulated object.
    pub current_position: f32,
    /// Current velocity of the simulated object.
    pub current_velocity: f32,
    /// Target equilibrium position the spring seeks.
    pub target_position: f32,
    /// Flag indicating whether the simulation has converged to equilibrium and is at rest.
    pub is_at_rest: bool,
}

impl SpringSimulation {
    /// Creates a new spring simulation starting at `initial_position` seeking `target_position`.
    #[must_use]
    pub fn new(config: SpringConfig, initial_position: f32, target_position: f32) -> Self {
        let is_at_rest = (initial_position - target_position).abs() < config.position_tolerance;
        Self {
            config,
            current_position: initial_position,
            current_velocity: 0.0,
            target_position,
            is_at_rest,
        }
    }

    /// Builder method to specify an initial velocity.
    #[must_use]
    pub fn with_initial_velocity(mut self, velocity: f32) -> Self {
        self.current_velocity = velocity;
        if velocity.abs() >= self.config.velocity_tolerance {
            self.is_at_rest = false;
        }
        self
    }

    /// Sets a new target equilibrium position without interrupting current velocity.
    pub fn set_target(&mut self, new_target: f32) {
        if (self.target_position - new_target).abs() > f32::EPSILON {
            self.target_position = new_target;
            self.is_at_rest = false;
        }
    }

    /// Resets the simulation state immediately with new position, target, and velocity.
    pub fn reset(&mut self, position: f32, target: f32, velocity: f32) {
        self.current_position = position;
        self.target_position = target;
        self.current_velocity = velocity;
        self.check_rest_state();
    }

    /// Advances the simulation by `dt` seconds using exact analytical integration.
    /// Returns the updated current position.
    pub fn step(&mut self, dt: f32) -> f32 {
        if self.is_at_rest || dt <= 0.0 {
            return self.current_position;
        }

        let m = self.config.mass.max(0.0001);
        let k = self.config.stiffness.max(0.0001);
        let c = self.config.damping.max(0.0);

        let omega_0 = (k / m).sqrt();
        let zeta = c / (2.0 * (m * k).sqrt());

        let y0 = self.current_position - self.target_position;
        let v0 = self.current_velocity;

        let (new_y, new_v) = if (zeta - 1.0).abs() < 1e-4 {
            // Case 1: Critically Damped (zeta == 1.0)
            let decay = (-omega_0 * dt).exp();
            let c2 = v0 + omega_0 * y0;
            let pos_y = (y0 + c2 * dt) * decay;
            let vel_y = (v0 - omega_0 * c2 * dt) * decay;
            (pos_y, vel_y)
        } else if zeta < 1.0 {
            // Case 2: Underdamped (zeta < 1.0)
            let omega_d = omega_0 * (1.0 - zeta * zeta).sqrt();
            let alpha = zeta * omega_0;
            let decay = (-alpha * dt).exp();

            let c1 = y0;
            let c2 = (v0 + alpha * y0) / omega_d;

            let cos_term = (omega_d * dt).cos();
            let sin_term = (omega_d * dt).sin();

            let pos_y = decay * (c1 * cos_term + c2 * sin_term);
            let vel_y = decay * (v0 * cos_term - (omega_d * c1 + alpha * c2) * sin_term);
            (pos_y, vel_y)
        } else {
            // Case 3: Overdamped (zeta > 1.0)
            let gamma = omega_0 * (zeta * zeta - 1.0).sqrt();
            let r1 = -zeta * omega_0 - gamma;
            let r2 = -zeta * omega_0 + gamma;

            let c2 = (v0 - r1 * y0) / (r2 - r1);
            let c1 = y0 - c2;

            let exp1 = (r1 * dt).exp();
            let exp2 = (r2 * dt).exp();

            let pos_y = c1 * exp1 + c2 * exp2;
            let vel_y = r1 * c1 * exp1 + r2 * c2 * exp2;
            (pos_y, vel_y)
        };

        self.current_position = self.target_position + new_y;
        self.current_velocity = new_v;

        self.check_rest_state();
        self.current_position
    }

    fn check_rest_state(&mut self) {
        let displacement = (self.current_position - self.target_position).abs();
        let speed = self.current_velocity.abs();

        if displacement < self.config.position_tolerance && speed < self.config.velocity_tolerance {
            self.current_position = self.target_position;
            self.current_velocity = 0.0;
            self.is_at_rest = true;
        } else {
            self.is_at_rest = false;
        }
    }

    /// Returns `true` if the simulation has converged and is stationary at the target.
    #[must_use]
    pub const fn is_at_rest(&self) -> bool {
        self.is_at_rest
    }

    /// Returns the current simulated position.
    #[must_use]
    pub const fn position(&self) -> f32 {
        self.current_position
    }

    /// Returns the current velocity.
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.current_velocity
    }

    /// Returns the target destination position.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target_position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_critically_damped_convergence() {
        // mass 1.0, stiffness 100.0, damping 20.0 -> zeta = 20 / (2 * 10) = 1.0
        let config = SpringConfig {
            mass: 1.0,
            stiffness: 100.0,
            damping: 20.0,
            position_tolerance: 0.01,
            velocity_tolerance: 0.05,
        };

        let mut spring = SpringSimulation::new(config, 0.0, 100.0);
        assert!(!spring.is_at_rest());

        let dt = 1.0 / 60.0;
        let mut steps = 0;
        while !spring.is_at_rest() && steps < 300 {
            spring.step(dt);
            steps += 1;
        }

        assert!(spring.is_at_rest());
        assert!((spring.position() - 100.0).abs() < 1e-4);
        assert_eq!(spring.velocity(), 0.0);
    }

    #[test]
    fn test_spring_underdamped_oscillates_and_settles() {
        // Underdamped: damping 8.0, stiffness 100.0 -> zeta = 8 / 20 = 0.4
        let config = SpringConfig {
            mass: 1.0,
            stiffness: 100.0,
            damping: 8.0,
            position_tolerance: 0.02,
            velocity_tolerance: 0.1,
        };

        let mut spring = SpringSimulation::new(config, 0.0, 50.0);
        let dt = 1.0 / 120.0;
        let mut reached_overshoot = false;

        for _ in 0..600 {
            spring.step(dt);
            if spring.position() > 50.0 {
                reached_overshoot = true;
            }
            if spring.is_at_rest() {
                break;
            }
        }

        assert!(
            reached_overshoot,
            "Underdamped spring must oscillate past target"
        );
        assert!(spring.is_at_rest());
        assert!((spring.position() - 50.0).abs() < 1e-4);
    }

    #[test]
    fn test_spring_overdamped_smooth_arrival() {
        // Overdamped: damping 35.0, stiffness 100.0 -> zeta = 35 / 20 = 1.75
        let config = SpringConfig {
            mass: 1.0,
            stiffness: 100.0,
            damping: 35.0,
            position_tolerance: 0.05,
            velocity_tolerance: 0.1,
        };

        let mut spring = SpringSimulation::new(config, 0.0, 200.0);
        let dt = 1.0 / 60.0;

        for _ in 0..600 {
            let prev_pos = spring.position();
            spring.step(dt);
            // Overdamped should approach monotonically without crossing 200.0
            assert!(spring.position() >= prev_pos);
            assert!(spring.position() <= 200.0);
            if spring.is_at_rest() {
                break;
            }
        }

        assert!(spring.is_at_rest());
        assert!((spring.position() - 200.0).abs() < 1e-4);
    }

    #[test]
    fn test_spring_with_initial_velocity_and_retargeting() {
        let config = SpringConfig::snappy();
        let mut spring = SpringSimulation::new(config, 0.0, 100.0).with_initial_velocity(500.0);
        assert!(!spring.is_at_rest());

        spring.step(1.0 / 60.0);
        assert!(spring.position() > 0.0);

        // Dynamically change target while in flight
        spring.set_target(150.0);
        assert_eq!(spring.target(), 150.0);
        assert!(!spring.is_at_rest());
    }
}
