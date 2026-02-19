use ahash::RandomState;
use rand::Rng;
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicU32, Ordering};

pub type HashMap<K, V> = StdHashMap<K, V, RandomState>;
pub type HashSet<K> = StdHashSet<K, RandomState>;
pub const TWO_PI: f32 = PI * 2.0;

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

#[macro_export]
macro_rules! ternary {
    ($condition:expr, $if_true:expr, $if_false:expr) => {
        if $condition { $if_true } else { $if_false }
    };
}

#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => {{
        use std::collections::HashSet;
        use std::sync::{LazyLock, Mutex};

        static SEEN: LazyLock<Mutex<HashSet<String>>> =
            LazyLock::new(|| Mutex::new(HashSet::new()));

        let message = format!($($arg)+);
        let mut set = SEEN.lock().expect("warn_once mutex poisoned");

        if set.insert(message.clone()) {
            log::warn!("{}", message);
        }
    }};
}

#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => {{
        use std::collections::HashSet;
        use std::sync::{LazyLock, Mutex};

        static SEEN: LazyLock<Mutex<HashSet<String>>> =
            LazyLock::new(|| Mutex::new(HashSet::new()));

        let message = format!($($arg)+);
        let mut set = SEEN.lock().expect("debug_once mutex poisoned");

        if set.insert(message.clone()) {
            log::debug!("{}", message);
        }
    }};
}

#[macro_export]
macro_rules! debug_throttled {
    ($interval_ms:expr, $($arg:tt)*) => {{
        use std::collections::HashMap;
        use std::sync::{LazyLock, Mutex};
        use std::time::{Duration, Instant};

        static DEBUG_THROTTLE: LazyLock<Mutex<HashMap<&'static str, Instant>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));

        let key = stringify!($($arg)*);
        let interval = Duration::from_millis($interval_ms as u64);
        let mut throttle_map =
            DEBUG_THROTTLE.lock().expect("debug_throttled mutex poisoned");
        let now = Instant::now();

        let should_log = throttle_map
            .get(key)
            .map(|last| now.duration_since(*last) >= interval)
            .unwrap_or(true);

        if should_log {
            throttle_map.insert(key, now);
            log::debug!($($arg)*);
        }
    }};
}

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

pub fn bool_to_f32(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

pub fn map_range(
    value: f32,
    in_min: f32,
    in_max: f32,
    out_min: f32,
    out_max: f32,
) -> f32 {
    let input_span = in_max - in_min;
    if input_span.abs() <= f32::EPSILON {
        return out_min;
    }
    let t = (value - in_min) / input_span;
    out_min + t * (out_max - out_min)
}

pub mod constrain {
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.clamp(min, max)
    }

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

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

pub fn random_bool() -> bool {
    rand::random()
}

pub fn random_within_range_stepped(min: f32, max: f32, step: f32) -> f32 {
    let mut rng = rand::rng();
    let random_value = min + rng.random_range(0.0..1.0) * (max - min);
    let quantized_value = (random_value / step).round() * step;
    quantized_value.clamp(min, max)
}

pub fn safe_range(min: f32, max: f32) -> (f32, f32) {
    let a = if max < min { max } else { min };
    let mut b = if min > max { min } else { max };
    if a == b {
        b += f32::EPSILON;
    }
    (a, b)
}

pub fn uuid(length: usize) -> String {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const NUMBERS: &[u8] = b"0123456789";

    let mut rng = rand::rng();
    let mut out = String::with_capacity(length);

    for _ in 0..length {
        if random_bool() {
            let idx = rng.random_range(0..LETTERS.len());
            out.push(LETTERS[idx] as char);
        } else {
            let idx = rng.random_range(0..NUMBERS.len());
            out.push(NUMBERS[idx] as char);
        }
    }

    out
}

pub fn uuid_5() -> String {
    uuid(5)
}
