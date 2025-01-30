use std::f32::consts::PI;

use super::prelude::*;

pub fn str_to_fn_unary(name: &str) -> fn(f32) -> f32 {
    match name {
        "linear" => linear,
        "ease_in" => ease_in,
        "ease_out" => ease_out,
        "ease_in_out" => ease_in_out,
        "cubic_ease_in" => cubic_ease_in,
        "cubic_ease_out" => cubic_ease_out,
        "cubic_ease_in_out" => cubic_ease_in_out,
        "sine_ease_in" => sine_ease_in,
        "sine_ease_out" => sine_ease_out,
        "sine_ease_in_out" => sine_ease_in_out,
        "logarithmic" => logarithmic,
        _ => {
            loud_panic!(
                "Easing function '{}' not found or requires extra parameters.",
                name
            );
        }
    }
}

pub enum EasingFn {
    OneArg(fn(f32) -> f32),
    TwoArg(fn(f32, f32) -> f32),
}

pub fn str_to_fn_any(name: &str) -> EasingFn {
    match name {
        "linear" => EasingFn::OneArg(linear),
        "ease_in" => EasingFn::OneArg(ease_in),
        "ease_out" => EasingFn::OneArg(ease_out),
        "ease_in_out" => EasingFn::OneArg(ease_in_out),
        "cubic_ease_in" => EasingFn::OneArg(cubic_ease_in),
        "cubic_ease_out" => EasingFn::OneArg(cubic_ease_out),
        "cubic_ease_in_out" => EasingFn::OneArg(cubic_ease_in_out),
        "sine_ease_in" => EasingFn::OneArg(sine_ease_in),
        "sine_ease_out" => EasingFn::OneArg(sine_ease_out),
        "sine_ease_in_out" => EasingFn::OneArg(sine_ease_in_out),
        "logarithmic" => EasingFn::OneArg(logarithmic),
        "exponential" => EasingFn::TwoArg(exponential),
        "sigmoid" => EasingFn::TwoArg(sigmoid),
        _ => loud_panic!("Easing function '{}' not found.", name),
    }
}

pub fn linear(t: f32) -> f32 {
    t
}

pub fn ease_in(t: f32) -> f32 {
    t * t
}

pub fn ease_out(t: f32) -> f32 {
    t * (2.0 - t)
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

pub fn cubic_ease_in(t: f32) -> f32 {
    t * t * t
}

pub fn cubic_ease_out(t: f32) -> f32 {
    let u = t - 1.0;
    u * u * u + 1.0
}

pub fn cubic_ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let adjusted_t = t - 1.0;
        adjusted_t * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
    }
}

pub fn exponential(t: f32, exponent: f32) -> f32 {
    t.powf(exponent)
}

pub fn sine_ease_in(t: f32) -> f32 {
    1.0 - ((t * PI / 2.0).cos())
}

pub fn sine_ease_out(t: f32) -> f32 {
    (t * PI / 2.0).sin()
}

pub fn sine_ease_in_out(t: f32) -> f32 {
    0.5 * (1.0 - (PI * t).cos())
}

/// Suggested range [1, 10]
/// 1-5 Smooth
/// 5-15 = Balanced curves, noticable transition but not overly sharp
/// 15-20 = Very step curves, almost like a step function
pub fn sigmoid(t: f32, steepness: f32) -> f32 {
    1.0 / (1.0 + (-steepness * (t - 0.5)).exp())
}

pub fn logarithmic(t: f32) -> f32 {
    (1.0 + t * 9.0).ln() / 10.0f32.ln()
}
