use ahash::RandomState;
use nannou::prelude::*;
use nannou::rand::Rng;
use nannou::rand::rand;
use nannou::rand::thread_rng;
use std::collections::HashMap as StdHashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use super::prelude::*;

pub const TWO_PI: f32 = PI * 2.0;

pub type HashMap<K, V> = StdHashMap<K, V, RandomState>;

#[derive(Debug)]
pub struct AtomicF32 {
    inner: AtomicU32,
}

impl AtomicF32 {
    pub const fn new(value: f32) -> Self {
        Self {
            inner: AtomicU32::new(value.to_bits()),
        }
    }

    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.inner.load(order))
    }

    pub fn store(&self, value: f32, order: Ordering) {
        self.inner.store(value.to_bits(), order)
    }
}

/// `ternary!(cond, true_case, false_case)`
#[macro_export]
macro_rules! ternary {
    ($condition: expr, $_true: expr, $_false: expr) => {
        if $condition { $_true } else { $_false }
    };
}

pub fn bool_to_f32(cond: bool) -> f32 {
    ternary!(cond, 1.0, 0.0)
}

/// Utilities to contain a value within a range
pub mod constrain {
    /// Clamp a value between min and max
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        nannou::prelude::clamp(value, min, max)
    }

    /// Clamp a value between min and max such that values that overshoot are
    /// mirrored back in, e.g. `constrain::fold(1.2, 0.0, 1.0) // => 0.8`
    pub fn fold(value: f32, min: f32, max: f32) -> f32 {
        if min == max {
            return min;
        }
        if value == max {
            return max;
        }

        let range = max - min;
        let value = value - min;
        let distance = value.abs();

        let cycles = (distance / range).floor();
        let remainder = distance % range;

        if cycles as i32 % 2 == 0 {
            if value >= 0.0 {
                min + remainder
            } else {
                max - remainder
            }
        } else if value >= 0.0 {
            max - remainder
        } else {
            min + remainder
        }
    }

    /// Clamp a value between min and max such that values that overshoot enter
    /// from the opposite bound  e.g. `constrain::fold(1.2, 0.0, 1.0) // => 0.2`
    pub fn wrap(value: f32, min: f32, max: f32) -> f32 {
        if min == max {
            return min;
        }
        if value == max {
            return max;
        }

        let range = max - min;
        let value = value - min;

        let wrapped = value - (value / range).floor() * range;
        min + wrapped
    }
}

/// Linear interpolation between two values. Returns a value between `start` and
/// `end` based on the interpolation parameter `t` (typically 0.0 to 1.0).
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

pub fn random_bool() -> bool {
    random()
}

pub fn random_within_range_stepped(min: f32, max: f32, step: f32) -> f32 {
    let mut rng = rand::thread_rng();
    let random_value = min + rng.gen_range(0.0..1.0) * (max - min);
    let quantized_value = (random_value / step).round() * step;
    f32::max(min, f32::min(max, quantized_value))
}

/// A helper to avoid [`std::ops::Range`] errors when min > max by swapping min
/// if min is greater or adding an epsilon to whichever is greater to avoid the
/// error.
pub fn safe_range(min: f32, max: f32) -> (f32, f32) {
    let a = if max < min { max } else { min };
    let mut b = if min > max { min } else { max };
    if a == b {
        b += f32::EPSILON;
    }
    (a, b)
}

pub(crate) fn set_window_position(
    app: &App,
    window_id: window::Id,
    x: i32,
    y: i32,
) {
    app.window(window_id)
        .unwrap()
        .winit_window()
        .set_outer_position(nannou::winit::dpi::PhysicalPosition::new(x, y));
}

pub(crate) fn set_window_size(
    window: &nannou::winit::window::Window,
    w: i32,
    h: i32,
) {
    let logical_size = nannou::winit::dpi::LogicalSize::new(w, h);
    window.set_inner_size(logical_size);
}

/// Helper to find a file that is adjacent to your sketch
///
/// # Example
///
/// ```rust
/// // in ./my/sketches/foo.rs
///
/// to_absolute_path(file!(), "bar.rs")
/// // => <absolute_path_to>/my/sketches/bar.rs
/// ```
pub fn to_absolute_path(
    caller_file: &str,
    relative_path: impl AsRef<std::path::Path>,
) -> PathBuf {
    PathBuf::from(caller_file)
        .parent()
        .expect("Failed to get parent directory")
        .join(relative_path.as_ref())
}

/// Naive uuid generator
pub fn uuid(length: usize) -> String {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
    const NUMBERS: &str = "0123456789";

    let mut rng = thread_rng();
    (0..length)
        .map(|_| {
            if random_bool() {
                LETTERS
                    .chars()
                    .nth(rng.gen_range(0..LETTERS.len()))
                    .unwrap()
            } else {
                NUMBERS
                    .chars()
                    .nth(rng.gen_range(0..NUMBERS.len()))
                    .unwrap()
            }
        })
        .collect()
}

pub(crate) fn uuid_5() -> String {
    uuid(5)
}

#[cfg(test)]
pub mod tests {
    #[macro_export]
    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr) => {
            assert!(
                ($a - $b).abs() < 0.001,
                "Values not approximately equal: {} and {}, difference: {}",
                $a,
                $b,
                ($a - $b).abs()
            );
        };
        ($a:expr, $b:expr, $epsilon:expr) => {
            assert!(
                ($a - $b).abs() < $epsilon,
                "Values not approximately equal:
                    {} and {}, difference: {}, tolerance: {}",
                $a,
                $b,
                ($a - $b).abs(),
                $epsilon
            );
        };
    }
}
