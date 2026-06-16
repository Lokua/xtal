//! Easing curves for shaping normalized interpolation.
//!
//! Most functions are based on the formulas collected by
//! [easings.net](https://github.com/ai/easings.net), which trace back to
//! Robert Penner's easing equations. Unless a function says otherwise, `t` is
//! expected to be a normalized value from `0.0` to `1.0`.

use std::f32::consts::PI;
use std::fmt::{Display, Formatter};
use std::result::Result;
use std::str::FromStr;

/// Named easing curve or custom easing callback.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Clone, Debug, PartialEq)]
pub enum Easing {
    /// Linear pass-through.
    Linear,

    /// Alias for [`Self::EaseInQuad`].
    #[doc(alias = "EaseInQuad")]
    EaseIn,

    /// Alias for [`Self::EaseOutQuad`].
    #[doc(alias = "EaseOutQuad")]
    EaseOut,

    /// Alias for [`Self::EaseInOutQuad`].
    #[doc(alias = "EaseInOutQuad")]
    EaseInOut,

    /// Quadratic ease-in.
    EaseInQuad,
    /// Quadratic ease-out.
    EaseOutQuad,
    /// Quadratic ease-in/ease-out.
    EaseInOutQuad,
    /// Cubic ease-in.
    EaseInCubic,
    /// Cubic ease-out.
    EaseOutCubic,
    /// Cubic ease-in/ease-out.
    EaseInOutCubic,
    /// Quartic ease-in.
    EaseInQuart,
    /// Quartic ease-out.
    EaseOutQuart,
    /// Quartic ease-in/ease-out.
    EaseInOutQuart,
    /// Quintic ease-in.
    EaseInQuint,
    /// Quintic ease-out.
    EaseOutQuint,
    /// Quintic ease-in/ease-out.
    EaseInOutQuint,
    /// Sinusoidal ease-in.
    EaseInSine,
    /// Sinusoidal ease-out.
    EaseOutSine,
    /// Sinusoidal ease-in/ease-out.
    EaseInOutSine,
    /// Exponential ease-in.
    EaseInExpo,
    /// Exponential ease-out.
    EaseOutExpo,
    /// Exponential ease-in/ease-out.
    EaseInOutExpo,
    /// Circular ease-in.
    EaseInCirc,
    /// Circular ease-out.
    EaseOutCirc,
    /// Circular ease-in/ease-out.
    EaseInOutCirc,
    /// Back ease-in with overshoot.
    EaseInBack,
    /// Back ease-out with overshoot.
    EaseOutBack,
    /// Back ease-in/ease-out with overshoot.
    EaseInOutBack,
    /// Elastic ease-in with overshoot.
    EaseInElastic,
    /// Elastic ease-out with overshoot.
    EaseOutElastic,
    /// Elastic ease-in/ease-out with overshoot.
    EaseInOutElastic,
    /// Bounce ease-in.
    EaseInBounce,
    /// Bounce ease-out.
    EaseOutBounce,
    /// Bounce ease-in/ease-out.
    EaseInOutBounce,
    /// Logarithmic curve normalized to `0.0..=1.0`.
    Logarithmic,

    /// Caller-provided unary easing function.
    Custom(fn(f32) -> f32),

    /// Power curve where larger exponents bias values toward zero.
    Exponential(f32),
    /// Symmetric curve controlled by curvature and max exponent.
    Curve(f32, f32),
    /// Logistic sigmoid with configurable steepness.
    Sigmoid(f32),
}

impl Easing {
    /// String names accepted by [`FromStr`] and displayed by [`Display`].
    pub const FUNCTION_NAMES: &[&str] = &[
        "linear",
        "ease_in",
        "ease_out",
        "ease_in_out",
        "ease_in_quad",
        "ease_out_quad",
        "ease_in_out_quad",
        "ease_in_cubic",
        "ease_out_cubic",
        "ease_in_out_cubic",
        "ease_in_quart",
        "ease_out_quart",
        "ease_in_out_quart",
        "ease_in_quint",
        "ease_out_quint",
        "ease_in_out_quint",
        "ease_in_sine",
        "ease_out_sine",
        "ease_in_out_sine",
        "ease_in_expo",
        "ease_out_expo",
        "ease_in_out_expo",
        "ease_in_circ",
        "ease_out_circ",
        "ease_in_out_circ",
        "ease_in_back",
        "ease_out_back",
        "ease_in_out_back",
        "ease_in_elastic",
        "ease_out_elastic",
        "ease_in_out_elastic",
        "ease_in_bounce",
        "ease_out_bounce",
        "ease_in_out_bounce",
        "logarithmic",
        "custom",
        "exponential",
        "curve",
        "sigmoid",
    ];

    /// Returns a dynamically filtered list of unary function names. Useful for
    /// cases when you are selecting easings dynamically and don't want to deal
    /// with edge cases of custom or parametric easings.
    pub fn unary_function_names() -> Vec<&'static str> {
        Self::FUNCTION_NAMES
            .iter()
            .copied()
            .filter(|&name| {
                name != "custom"
                    && name != "exponential"
                    && name != "sigmoid"
                    && name != "curve"
            })
            .collect()
    }

    /// Applies this easing curve to normalized input `t`.
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Linear => linear(t),
            Self::EaseIn | Self::EaseInQuad => ease_in_quad(t),
            Self::EaseOut | Self::EaseOutQuad => ease_out_quad(t),
            Self::EaseInOut | Self::EaseInOutQuad => ease_in_out_quad(t),
            Self::EaseInCubic => ease_in_cubic(t),
            Self::EaseOutCubic => ease_out_cubic(t),
            Self::EaseInOutCubic => ease_in_out_cubic(t),
            Self::EaseInQuart => ease_in_quart(t),
            Self::EaseOutQuart => ease_out_quart(t),
            Self::EaseInOutQuart => ease_in_out_quart(t),
            Self::EaseInQuint => ease_in_quint(t),
            Self::EaseOutQuint => ease_out_quint(t),
            Self::EaseInOutQuint => ease_in_out_quint(t),
            Self::EaseInSine => ease_in_sine(t),
            Self::EaseOutSine => ease_out_sine(t),
            Self::EaseInOutSine => ease_in_out_sine(t),
            Self::EaseInExpo => ease_in_expo(t),
            Self::EaseOutExpo => ease_out_expo(t),
            Self::EaseInOutExpo => ease_in_out_expo(t),
            Self::EaseInCirc => ease_in_circ(t),
            Self::EaseOutCirc => ease_out_circ(t),
            Self::EaseInOutCirc => ease_in_out_circ(t),
            Self::EaseInBack => ease_in_back(t),
            Self::EaseOutBack => ease_out_back(t),
            Self::EaseInOutBack => ease_in_out_back(t),
            Self::EaseInElastic => ease_in_elastic(t),
            Self::EaseOutElastic => ease_out_elastic(t),
            Self::EaseInOutElastic => ease_in_out_elastic(t),
            Self::EaseInBounce => ease_in_bounce(t),
            Self::EaseOutBounce => bounce_out(t),
            Self::EaseInOutBounce => ease_in_out_bounce(t),
            Self::Logarithmic => logarithmic(t),

            Self::Custom(f) => f(t),

            // Parametric
            Self::Exponential(power) => exponential(t, *power),
            Self::Curve(curvature, max_exponent) => {
                curve(t, *curvature, *max_exponent)
            }
            Self::Sigmoid(steepness) => sigmoid(t, *steepness),
        }
    }
}

impl FromStr for Easing {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "linear" => Ok(Self::Linear),
            "ease_in" => Ok(Self::EaseIn),
            "ease_out" => Ok(Self::EaseOut),
            "ease_in_out" => Ok(Self::EaseInOut),
            "ease_in_quad" => Ok(Self::EaseInQuad),
            "ease_out_quad" => Ok(Self::EaseOutQuad),
            "ease_in_out_quad" => Ok(Self::EaseInOutQuad),
            "ease_in_cubic" => Ok(Self::EaseInCubic),
            "ease_out_cubic" => Ok(Self::EaseOutCubic),
            "ease_in_out_cubic" => Ok(Self::EaseInOutCubic),
            "ease_in_quart" => Ok(Self::EaseInQuart),
            "ease_out_quart" => Ok(Self::EaseOutQuart),
            "ease_in_out_quart" => Ok(Self::EaseInOutQuart),
            "ease_in_quint" => Ok(Self::EaseInQuint),
            "ease_out_quint" => Ok(Self::EaseOutQuint),
            "ease_in_out_quint" => Ok(Self::EaseInOutQuint),
            "ease_in_sine" => Ok(Self::EaseInSine),
            "ease_out_sine" => Ok(Self::EaseOutSine),
            "ease_in_out_sine" => Ok(Self::EaseInOutSine),
            "ease_in_expo" => Ok(Self::EaseInExpo),
            "ease_out_expo" => Ok(Self::EaseOutExpo),
            "ease_in_out_expo" => Ok(Self::EaseInOutExpo),
            "ease_in_circ" => Ok(Self::EaseInCirc),
            "ease_out_circ" => Ok(Self::EaseOutCirc),
            "ease_in_out_circ" => Ok(Self::EaseInOutCirc),
            "ease_in_back" => Ok(Self::EaseInBack),
            "ease_out_back" => Ok(Self::EaseOutBack),
            "ease_in_out_back" => Ok(Self::EaseInOutBack),
            "ease_in_elastic" => Ok(Self::EaseInElastic),
            "ease_out_elastic" => Ok(Self::EaseOutElastic),
            "ease_in_out_elastic" => Ok(Self::EaseInOutElastic),
            "ease_in_bounce" => Ok(Self::EaseInBounce),
            "ease_out_bounce" => Ok(Self::EaseOutBounce),
            "ease_in_out_bounce" => Ok(Self::EaseInOutBounce),
            "logarithmic" => Ok(Self::Logarithmic),

            "custom" => unimplemented!(),

            "exponential" => Ok(Self::Exponential(2.0)),
            "curve" => Ok(Self::Curve(2.0, 5.0)),
            "sigmoid" => Ok(Self::Sigmoid(5.0)),

            _ => Err(format!("Unknown easing function: {}", name)),
        }
    }
}

impl Display for Easing {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease_in",
            Self::EaseOut => "ease_out",
            Self::EaseInOut => "ease_in_out",
            Self::EaseInQuad => "ease_in_quad",
            Self::EaseOutQuad => "ease_out_quad",
            Self::EaseInOutQuad => "ease_in_out_quad",
            Self::EaseInCubic => "ease_in_cubic",
            Self::EaseOutCubic => "ease_out_cubic",
            Self::EaseInOutCubic => "ease_in_out_cubic",
            Self::EaseInQuart => "ease_in_quart",
            Self::EaseOutQuart => "ease_out_quart",
            Self::EaseInOutQuart => "ease_in_out_quart",
            Self::EaseInQuint => "ease_in_quint",
            Self::EaseOutQuint => "ease_out_quint",
            Self::EaseInOutQuint => "ease_in_out_quint",
            Self::EaseInSine => "ease_in_sine",
            Self::EaseOutSine => "ease_out_sine",
            Self::EaseInOutSine => "ease_in_out_sine",
            Self::EaseInExpo => "ease_in_expo",
            Self::EaseOutExpo => "ease_out_expo",
            Self::EaseInOutExpo => "ease_in_out_expo",
            Self::EaseInCirc => "ease_in_circ",
            Self::EaseOutCirc => "ease_out_circ",
            Self::EaseInOutCirc => "ease_in_out_circ",
            Self::EaseInBack => "ease_in_back",
            Self::EaseOutBack => "ease_out_back",
            Self::EaseInOutBack => "ease_in_out_back",
            Self::EaseInElastic => "ease_in_elastic",
            Self::EaseOutElastic => "ease_out_elastic",
            Self::EaseInOutElastic => "ease_in_out_elastic",
            Self::EaseInBounce => "ease_in_bounce",
            Self::EaseOutBounce => "ease_out_bounce",
            Self::EaseInOutBounce => "ease_in_out_bounce",
            Self::Logarithmic => "logarithmic",

            Self::Custom(_) => "custom",

            Self::Exponential(_) => "exponential",
            Self::Curve(..) => "curve",
            Self::Sigmoid(_) => "sigmoid",
        };

        write!(f, "{}", s)
    }
}

const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.0;
const C4: f32 = (2.0 * PI) / 3.0;
const C5: f32 = (2.0 * PI) / 4.5;

/// Linear pass-through easing.
pub fn linear(t: f32) -> f32 {
    t
}

/// Quadratic ease-in.
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

/// Quadratic ease-out.
pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Quadratic ease-in/ease-out.
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Cubic ease-in.
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// Cubic ease-out.
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in/ease-out.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Quartic ease-in.
pub fn ease_in_quart(t: f32) -> f32 {
    t * t * t * t
}

/// Quartic ease-out.
pub fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

/// Quartic ease-in/ease-out.
pub fn ease_in_out_quart(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

/// Quintic ease-in.
pub fn ease_in_quint(t: f32) -> f32 {
    t * t * t * t * t
}

/// Quintic ease-out.
pub fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

/// Quintic ease-in/ease-out.
pub fn ease_in_out_quint(t: f32) -> f32 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
    }
}

/// Sinusoidal ease-in.
pub fn ease_in_sine(t: f32) -> f32 {
    1.0 - ((t * PI / 2.0).cos())
}

/// Sinusoidal ease-out.
pub fn ease_out_sine(t: f32) -> f32 {
    (t * PI / 2.0).sin()
}

/// Sinusoidal ease-in/ease-out.
pub fn ease_in_out_sine(t: f32) -> f32 {
    -(t * PI).cos() / 2.0 + 0.5
}

/// Exponential ease-in.
pub fn ease_in_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else {
        (2.0_f32).powf(10.0 * t - 10.0)
    }
}

/// Exponential ease-out.
pub fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - (2.0_f32).powf(-10.0 * t)
    }
}

/// Exponential ease-in/ease-out.
pub fn ease_in_out_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        (2.0_f32).powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0
    }
}

/// Circular ease-in.
pub fn ease_in_circ(t: f32) -> f32 {
    1.0 - (1.0 - t * t).sqrt()
}

/// Circular ease-out.
pub fn ease_out_circ(t: f32) -> f32 {
    ((1.0 - t) * (1.0 - t)).sqrt()
}

/// Circular ease-in/ease-out.
pub fn ease_in_out_circ(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
    }
}

/// Back ease-in with overshoot.
pub fn ease_in_back(t: f32) -> f32 {
    C3 * t * t * t - C1 * t * t
}

/// Back ease-out with overshoot.
pub fn ease_out_back(t: f32) -> f32 {
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Back ease-in/ease-out with overshoot.
pub fn ease_in_out_back(t: f32) -> f32 {
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0)
            / 2.0
    }
}

/// Elastic ease-in with overshoot.
pub fn ease_in_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        -(2.0_f32).powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * C4).sin()
    }
}

/// Elastic ease-out with overshoot.
pub fn ease_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
    }
}

/// Elastic ease-in/ease-out with overshoot.
pub fn ease_in_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        -((2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin())
            / 2.0
    } else {
        ((2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin())
            / 2.0
            + 1.0
    }
}

/// Bounce ease-in.
pub fn ease_in_bounce(t: f32) -> f32 {
    1.0 - bounce_out(1.0 - t)
}

/// Bounce ease-in/ease-out.
pub fn ease_in_out_bounce(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
    } else {
        (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
    }
}

fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;

    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Logarithmic curve normalized to `0.0..=1.0`.
pub fn logarithmic(t: f32) -> f32 {
    (1.0 + t * 9.0).ln() / 10.0f32.ln()
}

// -----------------------------------------------------------------------------
//  PARAMETRIC EASINGS
// -----------------------------------------------------------------------------

/// Raises `t` to `exponent`.
pub fn exponential(t: f32, exponent: f32) -> f32 {
    t.powf(exponent)
}

/// Default maximum exponent suggested for [`curve`].
pub const SUGGESTED_CURVE_MAX_EXPONENT: f32 = 10.0;

/// Creates a symmetric exponential easing function where the parameter controls
/// the curve in both directions from linear.
///
/// - `t`: input value, normally `0.0..=1.0`
/// - `curvature`: controls curve shape
/// - `max_exponent`: strongest exponent used at full curvature
///
/// # Example
///
/// - `curvature = 0.0`: linear curve
/// - `curvature = 1.0`: strong ease-out, biased toward max
/// - `curvature = -1.0`: strong ease-in, biased toward min
pub fn curve(t: f32, curvature: f32, max_exponent: f32) -> f32 {
    if curvature == 0.0 {
        return t;
    }

    if curvature > 0.0 {
        // Positive curvature: Ease-out effect
        // Map curvature range [0.0, 1.0] to exponent range [1.0, max_exponent]
        let normalized_curvature = curvature.min(1.0); // Clamp to max 1.0
        let exponent = 1.0 + normalized_curvature * (max_exponent - 1.0);
        1.0 - (1.0 - t).powf(exponent)
    } else {
        // Negative curvature: Ease-in effect (mirror of positive)
        // Use the absolute value to get the same exponent as the positive case
        let normalized_curvature = (-curvature).min(1.0); // Clamp to max 1.0
        let exponent = 1.0 + normalized_curvature * (max_exponent - 1.0);

        t.powf(exponent)
    }
}

/// Logistic sigmoid curve centered around `0.5`.
///
/// Suggested steepness ranges:
///
/// - `1..5`: smooth
/// - `5..15`: balanced, noticeable transition but not overly sharp
/// - `15..20`: very steep, almost like a step function
pub fn sigmoid(t: f32, steepness: f32) -> f32 {
    1.0 / (1.0 + (-steepness * (t - 0.5)).exp())
}
