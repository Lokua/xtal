//! Signal processing effects

use std::f32::consts::PI;

use super::prelude::*;

/// Limits the rate of change (slew rate) of a signal
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

    #[doc(alias = "slew")]
    pub fn apply(&mut self, value: f32) -> f32 {
        self.slew(value)
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
enum HysteresisState {
    High,
    Low,
}

/// Implements a Schmitt trigger with configurable thresholds that outputs:
/// - `output_high` when input rises above `upper_threshold`
/// - `output_low` when input falls below `lower_threshold`
/// - previous output when input is between thresholds
/// - input value when between thresholds and `pass_through` is true
pub struct Hysteresis {
    /// When true, allows values that are between the upper and lower thresholds
    /// to pass through. When false, binary hysteresis is applied
    pub pass_through: bool,
    pub upper_threshold: f32,
    pub lower_threshold: f32,

    /// The value to output when input is above the upper threshold`
    pub output_high: f32,

    /// The value to output when input is below the lower threshold
    pub output_low: f32,
    state: HysteresisState,
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

/// ⚠️ Experimental
#[derive(Debug, Clone)]
pub struct WaveFolder {
    /// Suggested range: 0.0 to 10.0
    /// - 0.0-1.0: gain reduction
    /// - 1.0: unity gain
    /// - 2.0-4.0: typical folding range
    /// - 4.0-10.0: extreme folding
    pub gain: f32,

    /// Suggested range: 1 to 8
    /// - 1-2: subtle harmonics
    /// - 3-4: moderate complexity
    /// - 5+: extreme/digital sound
    pub iterations: usize,

    /// changes the relative intensity of folding above vs below the center
    /// point by scaling the positive and negative portions differently.
    ///
    /// Suggested range: 0.5 to 2.0
    /// - 1.0: perfectly symmetric
    /// - <1.0: negative side folds less
    /// - >1.0: negative side folds more
    pub symmetry: f32,

    /// Shifts the center point of folding, effectively moving the "zero
    /// crossing" point.
    ///
    /// Suggested range: -1.0 to 1.0
    /// - 0.0: no DC offset
    /// - ±0.1-0.3: subtle asymmetry
    /// - ±0.5-1.0: extreme asymmetry
    pub bias: f32,

    /// Suggested range: -2.0 to 2.0 (values below -2.0 are hard capped)
    /// - < 0.0: softer folding curves
    /// - -1.0: perfectly sine-shaped folds
    /// - < -2.0: introduces intermediary folds but slight loss in overall
    ///   amplitude around ~-2.5
    /// - 1.0: linear folding
    /// - >1.0: sharper folding edges
    pub shape: f32,

    /// The (assumed) domain and range of the input and output signal
    range: (f32, f32),
}

impl WaveFolder {
    pub fn new(
        gain: f32,
        iterations: usize,
        symmetry: f32,
        bias: f32,
        shape: f32,
        range: (f32, f32),
    ) -> Self {
        WaveFolder {
            gain,
            iterations,
            symmetry,
            bias,
            shape,
            range,
        }
    }

    #[doc(alias = "fold")]
    pub fn apply(&self, input: f32) -> f32 {
        let mut output = input;
        for _ in 0..self.iterations {
            output = self.fold_once(output);
        }
        output
    }

    pub fn fold(&self, input: f32) -> f32 {
        self.apply(input)
    }

    pub fn set_range(&mut self, range: (f32, f32)) {
        self.range = range;
    }

    fn fold_once(&self, input: f32) -> f32 {
        // Comments assume the following settings unless noted otherwise:
        // - input: 0.7
        // - range: [0, 1]
        // - gain: 2.0
        // - bias: 0.0 (none)
        // - symmetry: 1.0 (symmetric)
        // - shape: 0.0 (linear)
        // ---------------------
        let (min, max) = self.range;

        let range = max - min; // 1.0

        // Center around 0.0 by subtracting the midpoint
        // [0, 1] becomes [-0.5, 0.5]

        // 0.5
        let half_range = range / 2.0;
        // 0.5
        let midpoint = min + half_range;
        // 0.7 - 0.5 = 0.2
        let centered = input - midpoint;

        // 0.2 * 2.0 = 0.4
        let amped = centered * self.gain;

        // 0.4 / 0.5 = 0.8
        let normalized = amped / half_range;

        // Apply bias to shift the folding center
        // 0.8 + 0.0 = 0.8
        let biased = normalized + self.bias;

        // Apply asymmetry before folding
        let asymmetric = if normalized > 0.0 {
            // 0.8 * 1.0 = 0.8
            biased * self.symmetry
        } else {
            biased / self.symmetry
        };

        // The folding logic

        // floor(0.8) = 0
        let cycles = asymmetric.abs().floor() as i32;
        // 0.8 - 0 = 0.8
        let remainder = asymmetric.abs() - cycles as f32;
        let pre_shaped = if cycles % 2 == 0 {
            // 0.8 * 1.0 = 0.8
            remainder * asymmetric.signum()
        } else {
            (1.0 - remainder) * asymmetric.signum()
        };

        // Apply shaping - negative values smooth, positive values sharpen
        let shaped = if self.shape < 0.0 {
            // Smoother folds using sine, scaled by abs(shape)
            let sine_shaped = (pre_shaped * PI / 2.0).sin();
            if self.shape < -1.0 {
                // Cap at -2.0. Values below "explode"
                let intensity = (-self.shape).min(2.0);
                let extra_shape = (pre_shaped * PI * intensity).sin();

                // Blend while maintaining as much amplitude as possible
                sine_shaped * (2.0 - intensity)
                    + extra_shape * (intensity - 1.0)
            } else {
                // Original smooth blend for -1.0 to 0.0
                pre_shaped * (1.0 + self.shape) + sine_shaped * (-self.shape)
            }
        } else if self.shape > 0.0 {
            let power = 1.0 + self.shape;
            pre_shaped.abs().powf(power) * pre_shaped.signum()
        } else {
            // Linear at 0.0
            pre_shaped
        };

        // 0.8 * 0.5 + 0.5 = 0.8
        shaped * half_range + midpoint
    }
}

impl Default for WaveFolder {
    fn default() -> Self {
        Self {
            gain: 1.0,
            iterations: 1,
            // Symmetric folding
            symmetry: 1.0,
            // No DC offset
            bias: 0.0,
            // Linear folding
            shape: 1.0,
            range: (0.0, 1.0),
        }
    }
}

/// Discretizes continuous input values into fixed steps, creating stair-case
/// transitions.
///
/// For example, with a step size of 0.25 in range (0.0, 1.0):
/// - Input 0.12 -> Output 0.0
/// - Input 0.26 -> Output 0.25
/// - Input 0.51 -> Output 0.50
#[derive(Debug, Clone)]
pub struct Quantizer {
    /// The size of each discrete step
    pub step: f32,

    /// The (assumed) domain and range of the input and output signal
    range: (f32, f32),
}

impl Quantizer {
    pub fn new(step: f32, range: (f32, f32)) -> Self {
        Self { step, range }
    }

    #[doc(alias = "quantize")]
    pub fn apply(&self, input: f32) -> f32 {
        self.quantize(input)
    }

    pub fn quantize(&self, input: f32) -> f32 {
        let (min, max) = self.range;
        let range = max - min;
        let normalized = (input - min) / range;
        let steps = (normalized / self.step).round();
        // Convert back to step-based value and denormalize
        let quantized = (steps * self.step) * range + min;
        quantized.clamp(min, max)
    }

    pub fn set_range(&mut self, range: (f32, f32)) {
        self.range = range;
    }
}

impl Default for Quantizer {
    fn default() -> Self {
        Self {
            step: 0.25,
            range: (0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Quantizer;
    use super::WaveFolder;
    use crate::framework::util::tests::approx_eq;

    #[test]
    fn test_wave_folder() {
        let wf = WaveFolder::default();
        approx_eq(wf.fold(1.2), 0.8);
    }

    #[test]
    fn test_wave_folder_gain() {
        let wf = WaveFolder::new(2.0, 1, 1.0, 0.0, 0.0, (0.0, 1.0));
        approx_eq(wf.fold(0.5), 1.0);
    }

    #[test]
    fn test_wave_folder_comments_case() {
        let wf = WaveFolder::new(2.0, 1, 1.0, 0.0, 0.0, (0.0, 1.0));
        approx_eq(wf.fold(0.7), 0.8);
    }

    #[test]
    fn test_quantizer_default() {
        let quantizer = Quantizer::default();
        approx_eq(quantizer.quantize(0.12), 0.0);
        approx_eq(quantizer.quantize(0.26), 0.25);
        approx_eq(quantizer.quantize(0.51), 0.50);
        approx_eq(quantizer.quantize(0.88), 1.0);
    }

    #[test]
    fn test_quantizer_custom() {
        let quantizer = Quantizer::new(0.2, (-1.0, 1.0));
        approx_eq(quantizer.quantize(0.3), 0.2);
        approx_eq(quantizer.quantize(-0.35), -0.4);
        approx_eq(quantizer.quantize(0.95), 1.0);
    }
}
