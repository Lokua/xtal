use super::prelude::*;

pub struct SlewLimiter {
    /// Controls smoothing when signal amplitude increases.
    /// - 0.0 = instant attack (no smoothing)
    /// - 1.0 = very slow attack (maximum smoothing)
    rise: f32,

    /// Controls smoothing when signal amplitude decreases.
    /// - 0.0 = instant decay (no smoothing)
    /// - 1.0 = very slow decay (maximum smoothing)
    fall: f32,

    previous_value: f32,
}

impl SlewLimiter {
    pub fn new(previous_value: f32, rise: f32, fall: f32) -> Self {
        Self {
            previous_value,
            rise,
            fall,
        }
    }

    pub fn slew(&mut self, value: f32) -> f32 {
        self.slew_with_rates(value, self.rise, self.fall)
    }

    /// Stateful version that takes new rates but doesn't save them
    pub fn slew_with_rates(&mut self, value: f32, rise: f32, fall: f32) -> f32 {
        let slewed = Self::slew_pure(self.previous_value, value, rise, fall);
        self.previous_value = slewed;
        slewed
    }

    pub fn set_rates(&mut self, rise: f32, fall: f32) {
        self.rise = rise;
        self.fall = fall;
    }

    pub fn slew_pure(
        previous_value: f32,
        value: f32,
        rise: f32,
        fall: f32,
    ) -> f32 {
        let coeff = ternary!(value > previous_value, 1.0 - rise, 1.0 - fall);
        previous_value + coeff * (value - previous_value)
    }
}

impl Default for SlewLimiter {
    fn default() -> Self {
        Self {
            previous_value: 0.0,
            rise: 0.0,
            fall: 0.0,
        }
    }
}
