//! Beat-oriented animation, easing, effects, and transport timing utilities.
//!
//! `motion` is the high-level animation layer used by sketches and control
//! scripts. It converts timing sources into beat positions, generates
//! repeatable animation values, shapes transitions with easing curves, and
//! post-processes animation output with small signal effects.

/// Musically timed animation helpers and automation breakpoints.
pub mod animation;
/// Easing curves used by animation ramps and control scripts.
pub mod easing;
/// Signal effects for shaping animation and control values.
pub mod effects;
/// Beat timing sources backed by frame, OSC, MIDI, hybrid, or manual clocks.
pub mod timing;

/// Re-export animation helpers for `crate::motion::*` callers.
pub use animation::*;
/// Re-export easing curves for `crate::motion::*` callers.
pub use easing::*;
/// Re-export signal effects for `crate::motion::*` callers.
pub use effects::*;
/// Re-export timing sources for `crate::motion::*` callers.
pub use timing::*;
