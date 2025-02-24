//! Signal processing effects designed to operate on the results of
//! [`Animation`][animation] methods but may be suitable for other domains.
//!
//! [animation]: crate::framework::animation

use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, PI};
use std::str::FromStr;

use super::prelude::*;

#[derive(Debug)]
pub enum Effect {
    Hysteresis(Hysteresis),
    Math(Math),
    Quantizer(Quantizer),
    RingModulator(RingModulator),
    Saturator(Saturator),
    SlewLimiter(SlewLimiter),
    WaveFolder(WaveFolder),
}

#[derive(Debug, PartialEq, Clone)]
enum HysteresisState {
    High,
    Low,
}

/// Implements a Schmitt trigger with configurable thresholds that outputs:
/// - `output_high` when input rises above `upper_threshold`
/// - `output_low` when input falls below `lower_threshold`
/// - previous output when input is between thresholds
/// - input value when between thresholds and `pass_through` is true
#[derive(Debug, Clone)]
pub struct Hysteresis {
    /// When true, allows values that are between the upper and lower thresholds
    /// to pass through. When false, binary hysteresis is applied
    pub pass_through: bool,
    pub upper_threshold: f32,
    pub lower_threshold: f32,

    /// The value to output when input is above the upper threshold
    pub output_high: f32,

    /// The value to output when input is below the lower threshold
    pub output_low: f32,
    state: RefCell<HysteresisState>,
}

impl Hysteresis {
    pub fn new(
        lower_threshold: f32,
        upper_threshold: f32,
        output_low: f32,
        output_high: f32,
        pass_through: bool,
    ) -> Self {
        let (lower_threshold, upper_threshold) =
            safe_range(lower_threshold, upper_threshold);
        Self {
            state: RefCell::new(HysteresisState::Low),
            lower_threshold,
            upper_threshold,
            output_low,
            output_high,
            pass_through,
        }
    }

    pub fn apply(&self, input: f32) -> f32 {
        if input >= self.upper_threshold {
            self.state.replace(HysteresisState::High);
        } else if input <= self.lower_threshold {
            self.state.replace(HysteresisState::Low);
        } else if self.pass_through {
            return input;
        }
        ternary!(
            *self.state.borrow() == HysteresisState::Low,
            self.output_low,
            self.output_high
        )
    }
}

impl Default for Hysteresis {
    fn default() -> Self {
        Self {
            lower_threshold: 0.3,
            upper_threshold: 0.7,
            output_low: 0.0,
            output_high: 0.0,
            pass_through: false,
            state: RefCell::new(HysteresisState::Low),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Mult,
}

impl FromStr for Operator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Operator::Add),
            "mult" => Ok(Operator::Mult),
            _ => Err(format!("No op named {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Math {
    pub operator: Operator,
    pub operand: f32,
}

impl Math {
    pub fn new(op: Operator, value: f32) -> Self {
        Self {
            operator: op,
            operand: value,
        }
    }

    pub fn apply(&self, input: f32) -> f32 {
        match self.operator {
            Operator::Add => self.operand + input,
            Operator::Mult => self.operand * input,
        }
    }
}

impl Default for Math {
    fn default() -> Self {
        Self {
            operator: Operator::Add,
            operand: 1.0,
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

/// Implements ring modulation by combining a carrier and modulator signal.
#[derive(Debug, Clone)]
pub struct RingModulator {
    /// Controls the blend between carrier and modulated signal
    /// - 0.0: outputs carrier signal
    /// - 0.5: outputs true ring modulation (carrier * modulator)
    /// - 1.0: outputs modulator signal
    pub mix: f32,

    /// The (assumed) domain and range of the input and output signal
    range: (f32, f32),
}

impl RingModulator {
    pub fn new(depth: f32, range: (f32, f32)) -> Self {
        Self { mix: depth, range }
    }

    #[doc(alias = "modulate")]
    pub fn apply(&self, carrier: f32, modulator: f32) -> f32 {
        self.modulate(carrier, modulator)
    }

    pub fn modulate(&self, carrier: f32, modulator: f32) -> f32 {
        let (min, max) = self.range;
        let range = max - min;
        let midpoint = min + range / 2.0;

        // Center signals around zero for multiplication
        // Scale to -1 to +1
        let carrier_centered = (carrier - midpoint) * 2.0;
        // Scale to -1 to +1
        let modulator_centered = (modulator - midpoint) * 2.0;

        let ring_mod = carrier_centered * modulator_centered;

        // Interpolate between carrier, ring mod, and modulator based on depth
        let result = if self.mix <= 0.5 {
            // Blend between carrier (0.0) and ring mod (0.5)
            let t = self.mix * 2.0;
            carrier_centered * (1.0 - t) + ring_mod * t
        } else {
            // Blend between ring mod (0.5) and modulator (1.0)
            let t = (self.mix - 0.5) * 2.0;
            ring_mod * (1.0 - t) + modulator_centered * t
        };

        ((result / 2.0) + midpoint).clamp(min, max)
    }

    pub fn set_range(&mut self, range: (f32, f32)) {
        self.range = range;
    }
}

impl Default for RingModulator {
    fn default() -> Self {
        Self {
            mix: 0.5,
            range: (0.0, 1.0),
        }
    }
}

/// Applies smooth saturation to a signal, creating a soft roll-off as values
/// approach the range boundaries. Higher drive values create more aggressive
/// saturation effects.
///
/// Note: WIP - this is just tanh clipping at this point
#[derive(Debug, Clone)]
pub struct Saturator {
    /// Controls the intensity of the saturation effect. Higher values push more
    /// of the signal into the saturated region.
    /// - 0.0: no saturation (pure pass-through)
    /// - >0.0 & <1.0: experimental WIP easing between dry signal and saturation
    /// - 1.0: subtle saturation
    /// - 2.0-4.0: moderate saturation
    /// - 4.0+: aggressive saturation
    pub drive: f32,

    /// The (assumed) domain and range of the input and output signal
    range: (f32, f32),
}

impl Saturator {
    pub fn new(drive: f32, range: (f32, f32)) -> Self {
        Self { drive, range }
    }

    #[doc(alias = "saturate")]
    pub fn apply(&self, input: f32) -> f32 {
        self.saturate(input)
    }

    pub fn saturate(&self, input: f32) -> f32 {
        if self.drive == 0.0 {
            return input;
        }
        let (min, max) = self.range;
        let range = max - min;
        let midpoint = min + range / 2.0;

        // Center around 0 and normalize to roughly -1 to 1
        let normalized = 2.0 * (input - midpoint) / range;

        let saturated = if self.drive < 1.0 {
            let saturated = normalized.tanh();
            let eased_drive = ease_out_expo(self.drive);
            normalized * (1.0 - eased_drive) + saturated * eased_drive
        } else {
            (normalized * self.drive).tanh()
        };

        // Denormalize and recenter
        saturated * (range / 2.0) + midpoint
    }

    pub fn set_range(&mut self, range: (f32, f32)) {
        self.range = range;
    }
}

impl Default for Saturator {
    fn default() -> Self {
        Self {
            drive: 1.0,
            range: (0.0, 1.0),
        }
    }
}

/// Limits the rate of change (slew rate) of a signal
#[derive(Debug, Clone)]
pub struct SlewLimiter {
    /// Controls smoothing when signal amplitude increases.
    /// - 0.0 = instant attack (no smoothing)
    /// - 1.0 = very slow attack (maximum smoothing)
    pub rise: f32,

    /// Controls smoothing when signal amplitude decreases.
    /// - 0.0 = instant decay (no smoothing)
    /// - 1.0 = very slow decay (maximum smoothing)
    pub fall: f32,

    previous_value: RefCell<f32>,
}

impl SlewLimiter {
    pub fn new(rise: f32, fall: f32) -> Self {
        Self {
            previous_value: RefCell::new(0.0),
            rise,
            fall,
        }
    }

    pub fn apply(&self, value: f32) -> f32 {
        self.slew(value)
    }

    #[doc(alias = "apply")]
    pub fn slew(&self, value: f32) -> f32 {
        self.slew_with_rates(value, self.rise, self.fall)
    }

    /// Stateful version that takes new rates but doesn't save them
    pub fn slew_with_rates(&self, value: f32, rise: f32, fall: f32) -> f32 {
        let slewed =
            Self::slew_pure(*self.previous_value.borrow(), value, rise, fall);
        self.previous_value.replace(slewed);
        slewed
    }

    pub fn slew_pure(
        previous_value: f32,
        value: f32,
        rise: f32,
        fall: f32,
    ) -> f32 {
        let coeff = 1.0
            - ternary!(
                value > previous_value,
                ease_in_out_expo(rise),
                ease_in_out_expo(fall)
            );
        previous_value + coeff * (value - previous_value)
    }

    pub fn set_rates(&mut self, rise: f32, fall: f32) {
        self.rise = rise;
        self.fall = fall;
    }
}

impl Default for SlewLimiter {
    fn default() -> Self {
        Self {
            previous_value: RefCell::new(0.0),
            rise: 0.0,
            fall: 0.0,
        }
    }
}

/// ⚠️ Experimental
#[derive(Debug, Clone)]
pub struct WaveFolder {
    /// Suggested range: 1.0 to 10.0
    /// - <1.0: Bypassed
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
        if self.gain <= 1.0 {
            return input;
        }
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

/// Assumes all parameters are within normalized range
pub fn equal_power_crossfade(a: f32, b: f32, mix: f32) -> f32 {
    let t = mix.clamp(0.0, 1.0);

    let a_gain = ((1.0 - t) * FRAC_PI_2).cos();
    let b_gain = (t * FRAC_PI_2).sin();

    a * a_gain + b * b_gain
}

#[cfg(test)]
mod tests {
    use super::Quantizer;
    use super::Saturator;
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

    #[test]
    fn test_saturator_center_unchanged() {
        let saturator = Saturator::default();
        // Center point should pass through unchanged
        approx_eq(saturator.saturate(0.5), 0.5);
    }

    #[test]
    fn test_saturator_symmetry() {
        let saturator = Saturator::new(2.0, (0.0, 1.0));
        let high = saturator.saturate(0.8);
        let low = saturator.saturate(0.2);
        // Should be equidistant from center
        approx_eq(0.5 - low, high - 0.5);
    }

    #[test]
    fn test_saturator_range() {
        let saturator = Saturator::new(4.0, (-1.0, 1.0));
        // Even with high drive, should stay within range
        assert!(saturator.saturate(2.0) <= 1.0);
        assert!(saturator.saturate(-2.0) >= -1.0);
    }
}
