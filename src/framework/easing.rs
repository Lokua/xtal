use std::f32::consts::PI;

/// Easing functions, most of which borrowed from
/// [easings.net](https://github.com/ai/easings.net), which in turn come from
/// [Robert Penner](http://robertpenner.com/easing/), the guy who _literally_
/// wrote the book on easings.

#[derive(Clone, Debug, PartialEq)]
pub enum Easing {
    Linear,

    #[doc(alias = "EaseInQuad")]
    EaseIn,

    #[doc(alias = "EaseOutQuad")]
    EaseOut,

    #[doc(alias = "EaseInOutQuad")]
    EaseInOut,

    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    Logarithmic,

    Custom(fn(f32) -> f32),

    // --- PARAMETRIC EASINGS
    Exponential(f32),
    Sigmoid(f32),
}

impl Easing {
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
                name != "custom" && name != "exponential" && name != "sigmoid"
            })
            .collect()
    }

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
            Self::Sigmoid(steepness) => sigmoid(t, *steepness),
        }
    }

    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "linear" => Some(Self::Linear),
            "ease_in" => Some(Self::EaseIn),
            "ease_out" => Some(Self::EaseOut),
            "ease_in_out" => Some(Self::EaseInOut),
            "ease_in_quad" => Some(Self::EaseInQuad),
            "ease_out_quad" => Some(Self::EaseOutQuad),
            "ease_in_out_quad" => Some(Self::EaseInOutQuad),
            "ease_in_cubic" => Some(Self::EaseInCubic),
            "ease_out_cubic" => Some(Self::EaseOutCubic),
            "ease_in_out_cubic" => Some(Self::EaseInOutCubic),
            "ease_in_quart" => Some(Self::EaseInQuart),
            "ease_out_quart" => Some(Self::EaseOutQuart),
            "ease_in_out_quart" => Some(Self::EaseInOutQuart),
            "ease_in_quint" => Some(Self::EaseInQuint),
            "ease_out_quint" => Some(Self::EaseOutQuint),
            "ease_in_out_quint" => Some(Self::EaseInOutQuint),
            "ease_in_sine" => Some(Self::EaseInSine),
            "ease_out_sine" => Some(Self::EaseOutSine),
            "ease_in_out_sine" => Some(Self::EaseInOutSine),
            "ease_in_expo" => Some(Self::EaseInExpo),
            "ease_out_expo" => Some(Self::EaseOutExpo),
            "ease_in_out_expo" => Some(Self::EaseInOutExpo),
            "ease_in_circ" => Some(Self::EaseInCirc),
            "ease_out_circ" => Some(Self::EaseOutCirc),
            "ease_in_out_circ" => Some(Self::EaseInOutCirc),
            "ease_in_back" => Some(Self::EaseInBack),
            "ease_out_back" => Some(Self::EaseOutBack),
            "ease_in_out_back" => Some(Self::EaseInOutBack),
            "ease_in_elastic" => Some(Self::EaseInElastic),
            "ease_out_elastic" => Some(Self::EaseOutElastic),
            "ease_in_out_elastic" => Some(Self::EaseInOutElastic),
            "ease_in_bounce" => Some(Self::EaseInBounce),
            "ease_out_bounce" => Some(Self::EaseOutBounce),
            "ease_in_out_bounce" => Some(Self::EaseInOutBounce),
            "logarithmic" => Some(Self::Logarithmic),

            "custom" => unimplemented!(),

            "exponential" => Some(Self::Exponential(2.0)),
            "sigmoid" => Some(Self::Sigmoid(5.0)),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
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
            Self::Sigmoid(_) => "sigmoid",
        }
    }
}

const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.0;
const C4: f32 = (2.0 * PI) / 3.0;
const C5: f32 = (2.0 * PI) / 4.5;

pub fn linear(t: f32) -> f32 {
    t
}

pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn ease_in_quart(t: f32) -> f32 {
    t * t * t * t
}

pub fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

pub fn ease_in_out_quart(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

pub fn ease_in_quint(t: f32) -> f32 {
    t * t * t * t * t
}

pub fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

pub fn ease_in_out_quint(t: f32) -> f32 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
    }
}

pub fn ease_in_sine(t: f32) -> f32 {
    1.0 - ((t * PI / 2.0).cos())
}

pub fn ease_out_sine(t: f32) -> f32 {
    (t * PI / 2.0).sin()
}

pub fn ease_in_out_sine(t: f32) -> f32 {
    -(t * PI).cos() / 2.0 + 0.5
}

pub fn ease_in_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else {
        (2.0_f32).powf(10.0 * t - 10.0)
    }
}

pub fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - (2.0_f32).powf(-10.0 * t)
    }
}

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

pub fn ease_in_circ(t: f32) -> f32 {
    1.0 - (1.0 - t * t).sqrt()
}

pub fn ease_out_circ(t: f32) -> f32 {
    ((1.0 - t) * (1.0 - t)).sqrt()
}

pub fn ease_in_out_circ(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
    }
}

pub fn ease_in_back(t: f32) -> f32 {
    C3 * t * t * t - C1 * t * t
}

pub fn ease_out_back(t: f32) -> f32 {
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

pub fn ease_in_out_back(t: f32) -> f32 {
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0)
            / 2.0
    }
}

pub fn ease_in_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        -(2.0_f32).powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * C4).sin()
    }
}

pub fn ease_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
    }
}

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

pub fn ease_in_bounce(t: f32) -> f32 {
    1.0 - bounce_out(1.0 - t)
}

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

pub fn logarithmic(t: f32) -> f32 {
    (1.0 + t * 9.0).ln() / 10.0f32.ln()
}

// --- PARAMETRIC EASINGS

pub fn exponential(t: f32, exponent: f32) -> f32 {
    t.powf(exponent)
}

/// Suggested range [1, 10]
/// 1-5 Smooth
/// 5-15 = Balanced curves, noticable transition but not overly sharp
/// 15-20 = Very step curves, almost like a step function
pub fn sigmoid(t: f32, steepness: f32) -> f32 {
    1.0 / (1.0 + (-steepness * (t - 0.5)).exp())
}
