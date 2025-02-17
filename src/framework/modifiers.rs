//! Signal processing effects

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

#[derive(PartialEq)]
pub enum HysteresisState {
    High,
    Low,
}

pub struct Hysteresis {
    state: HysteresisState,
    pass_through: bool,
    pub upper_threshold: f32,
    pub lower_threshold: f32,
    /// The value to output when state is [`HysteresisState::High`]
    pub output_high: f32,
    /// The value to output when state is [`HysteresisState::Low`]
    pub output_low: f32,
}

impl Hysteresis {
    pub fn new(
        lower_threshold: f32,
        upper_threshold: f32,
        output_low: f32,
        output_high: f32,
    ) -> Self {
        assert!(
            lower_threshold < upper_threshold,
            "Upper threshold must be greater than lower threshold"
        );
        Self {
            state: HysteresisState::Low,
            pass_through: false,
            lower_threshold,
            upper_threshold,
            output_low,
            output_high,
        }
    }

    pub fn with_pass_through(
        lower_threshold: f32,
        upper_threshold: f32,
        output_low: f32,
        output_high: f32,
    ) -> Self {
        let mut instance = Self::new(
            lower_threshold,
            upper_threshold,
            output_low,
            output_high,
        );
        instance.pass_through = true;
        instance
    }

    pub fn apply(&mut self, input: f32) -> f32 {
        if input >= self.upper_threshold {
            self.state = HysteresisState::High;
        } else if input <= self.lower_threshold {
            self.state = HysteresisState::Low;
        } else if self.pass_through {
            return input;
        }
        ternary!(
            self.state == HysteresisState::Low,
            self.output_low,
            self.output_high
        )
    }
}

impl Default for Hysteresis {
    fn default() -> Self {
        Hysteresis::new(0.3, 0.7, 0.0, 1.0)
    }
}
