use android_activity::input::Keycode;

pub struct TouchSlopFilter {
    pub touch_slop_px: f32,
    pub start_pos: Option<(f32, f32)>,
    pub is_slop_passed: bool,
}

impl TouchSlopFilter {
    #[must_use]
    pub fn new(touch_slop_px: f32) -> Self {
        Self {
            touch_slop_px,
            start_pos: None,
            is_slop_passed: false,
        }
    }

    pub fn start(&mut self, x: f32, y: f32) {
        self.start_pos = Some((x, y));
        self.is_slop_passed = false;
    }

    pub fn filter(&mut self, x: f32, y: f32) -> bool {
        if self.is_slop_passed {
            return true;
        }
        if let Some((sx, sy)) = self.start_pos {
            let dx = x - sx;
            let dy = y - sy;
            let dist_sq = dx.mul_add(dx, dy * dy);
            if dist_sq >= self.touch_slop_px * self.touch_slop_px {
                self.is_slop_passed = true;
                return true;
            }
        }
        false
    }
}

pub struct VelocityTracker {
    samples: std::collections::VecDeque<(f32, f32, std::time::Instant)>,
    max_samples: usize,
}

impl VelocityTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: std::collections::VecDeque::with_capacity(10),
            max_samples: 8,
        }
    }

    pub fn add_movement(&mut self, x: f32, y: f32) {
        let now = std::time::Instant::now();
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back((x, y, now));
    }

    #[must_use]
    pub fn compute_velocity(&self) -> (f32, f32) {
        if self.samples.len() < 2 {
            return (0.0, 0.0);
        }
        let first = self.samples.front().unwrap();
        let last = self.samples.back().unwrap();
        let dt = last.2.duration_since(first.2).as_secs_f32();
        if dt <= 0.001 {
            return (0.0, 0.0);
        }
        let vx = (last.0 - first.0) / dt;
        let vy = (last.1 - first.1) / dt;
        (vx, vy)
    }

    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

impl Default for VelocityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn keycode_to_char(keycode: Keycode) -> Option<char> {
    match keycode {
        Keycode::A => Some('a'),
        Keycode::B => Some('b'),
        Keycode::C => Some('c'),
        Keycode::D => Some('d'),
        Keycode::E => Some('e'),
        Keycode::F => Some('f'),
        Keycode::G => Some('g'),
        Keycode::H => Some('h'),
        Keycode::I => Some('i'),
        Keycode::J => Some('j'),
        Keycode::K => Some('k'),
        Keycode::L => Some('l'),
        Keycode::M => Some('m'),
        Keycode::N => Some('n'),
        Keycode::O => Some('o'),
        Keycode::P => Some('p'),
        Keycode::Q => Some('q'),
        Keycode::R => Some('r'),
        Keycode::S => Some('s'),
        Keycode::T => Some('t'),
        Keycode::U => Some('u'),
        Keycode::V => Some('v'),
        Keycode::W => Some('w'),
        Keycode::X => Some('x'),
        Keycode::Y => Some('y'),
        Keycode::Z => Some('z'),
        Keycode::Keycode0 => Some('0'),
        Keycode::Keycode1 => Some('1'),
        Keycode::Keycode2 => Some('2'),
        Keycode::Keycode3 => Some('3'),
        Keycode::Keycode4 => Some('4'),
        Keycode::Keycode5 => Some('5'),
        Keycode::Keycode6 => Some('6'),
        Keycode::Keycode7 => Some('7'),
        Keycode::Keycode8 => Some('8'),
        Keycode::Keycode9 => Some('9'),
        Keycode::Space => Some(' '),
        Keycode::Period => Some('.'),
        Keycode::Comma => Some(','),
        Keycode::Minus => Some('-'),
        _ => None,
    }
}
