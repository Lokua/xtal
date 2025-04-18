//! Timing, animation, and easing methods
pub mod animation;
pub use animation::*;

pub mod easing;
pub use easing::*;

pub mod effects;
pub use effects::*;

pub mod timing;
pub use timing::*;

#[cfg(test)]
pub use animation::animation_tests;
