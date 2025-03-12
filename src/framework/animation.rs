//! Animation module providing timing-synchronized animation and transition
//! controls.
//!
//! This module offers a flexible system for creating time-based animations that
//! are synchronized to musical time (beats) through various timing sources. The
//! core animation system supports:
//!
//! - Beat-synchronized timing with support for various clock sources (MIDI,
//!   OSC, manual)
//! - Simple oscillations and phase-based animations
//! - Linear and eased transitions between keyframes
//! - Complex automation curves with configurable breakpoints
//! - Randomized transitions with constraints
//!
//! # Musical Timing
//!
//! All animations are synchronized to musical time expressed in beats, where
//! one beat equals one quarter note. The animation system can be driven by
//! different timing sources (Frame, MIDI, OSC) that provide the current beat
//! position, allowing animations to stay in sync with external music software
//! or hardware.
//!
//! # Basic Usage
//!
//! ```rust
//! let animation = Animation::new(Timing::new(ctx.bpm()));
//!
//! // Simple oscillation between 0-1 over 4 beats
//! let phase = animation.loop_phase(4.0); // Returns 0.0 to 1.0
//!
//! // Triangle wave oscillation between ranges
//! let value = animation.triangle(
//!     // Duration in beats
//!     4.0,
//!     // Min/max range
//!     (0.0, 100.0),  
//!     // Phase offset
//!     0.0,           
//! );
//! ```
//!
//! # Advanced Automation
//!
//! The [`Animation::automate`] method provides DAW-style automation curves with
//! multiple breakpoint types and transition modes:
//!
//! ```rust
//! let value = animation.automate(
//!     &[
//!         // Start with a step change
//!         Breakpoint::step(0.0, 0.0),
//!         // Ramp with exponential easing
//!         Breakpoint::ramp(1.0, 1.0, Easing::EaseInExpo),
//!         // Add amplitude modulation
//!         Breakpoint::wave(
//!             // Position in beats
//!             2.0,
//!             // Base value
//!             0.5,
//!             Shape::Sine,
//!             // Frequency in beats
//!             0.25,
//!             // Width
//!             0.5,
//!             // Amplitude
//!             0.25,
//!             Easing::Linear,
//!             Constrain::None,
//!         ),
//!         // Mark end of sequence
//!         Breakpoint::end(4.0, 0.0),
//!     ],
//!     Mode::Loop
//! );
//! ```
//!
//! The module supports both one-shot and looping animations, with precise
//! control over timing, transitions, and modulation. All animations
//! automatically sync to the provided timing source's beat position.

use std::cell::RefCell;
use std::collections::HashMap;

use nannou::math::map_range;
use nannou::rand;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use super::frame_controller;
use super::prelude::*;

#[derive(Clone, Debug)]
pub struct KeyframeRandom {
    pub range: (f32, f32),
    pub duration: f32,
}

impl KeyframeRandom {
    pub fn new(range: (f32, f32), duration: f32) -> Self {
        Self { range, duration }
    }

    pub fn generate_value(&self, seed: u64) -> f32 {
        let mut rng = StdRng::seed_from_u64(seed);
        let random = rng.gen::<f32>();
        self.range.0 + (self.range.1 - self.range.0) * random
    }
}

/// Convenience method to create keyframes without
/// the verbosity of [`KeyframeRandom::new`]
pub fn kfr(range: (f32, f32), duration: f32) -> KeyframeRandom {
    KeyframeRandom::new(range, duration)
}

/// Data structure used in conjunction with
/// [`Animation::create_trigger`] and [`Animation::should_trigger`]
pub struct Trigger {
    every: f32,
    delay: f32,
    last_trigger_count: f32,
}

#[derive(Clone, Debug)]
pub struct Animation<T: TimingSource> {
    pub timing: T,
    random_smooth_previous_values: RefCell<HashMap<u64, f32>>,
}

impl<T: TimingSource> Animation<T> {
    pub fn new(timing: T) -> Self {
        Self {
            timing,
            random_smooth_previous_values: RefCell::new(HashMap::new()),
        }
    }

    /// Return the number of beats that have elapsed
    /// since (re)start of this Animation's Timing source
    pub fn beats(&self) -> f32 {
        self.timing.beats()
    }

    /// Convert `beats` to frame count
    pub fn beats_to_frames(&self, beats: f32) -> f32 {
        let seconds_per_beat = 60.0 / self.timing.bpm();
        let total_seconds = beats * seconds_per_beat;
        total_seconds * frame_controller::fps()
    }

    /// Return a relative phase position from [0, 1] within
    /// the passed in duration (specified in beats)
    pub fn loop_phase(&self, duration: f32) -> f32 {
        let total_beats = self.beats();
        (total_beats / duration) % 1.0
    }

    /// Cycle from 0 to 1 and back to 0 over the passed in duration
    /// See [`Self::triangle`] for an advanced version with more options
    pub fn tri(&self, duration: f32) -> f32 {
        let x = (self.beats() / duration) % 1.0;
        ternary!(x < 0.5, x, 1.0 - x) * 2.0
    }

    /// Cycle from `min` to `max` and back to `min` in exactly `duration`
    /// beats. `phase_offset` in [0.0..1.0] shifts our position in that cycle.
    /// Only positive offsets are supported.
    pub fn triangle(
        &self,
        duration: f32,
        (min, max): (f32, f32),
        phase_offset: f32,
    ) -> f32 {
        let mut x = (self.beats() / duration + phase_offset.abs() * 0.5) % 1.0;
        x = ternary!(x < 0.5, x, 1.0 - x) * 2.0;
        map_range(x, 0.0, 1.0, min, max)
    }

    /// Generate a randomized value once during every cycle of `duration`. The
    /// function is completely deterministic given the same parameters in
    /// relation to the current beat.
    pub fn random(
        &self,
        duration: f32,
        (min, max): (f32, f32),
        delay: f32,
        stem: u64,
    ) -> f32 {
        let beats = self.beats() - delay;
        let loop_count = ternary!(beats < 0.0, 0.0, (beats / duration).floor());
        let seed = stem + ((duration + (max - min) + loop_count) as u64);
        let mut rng = StdRng::seed_from_u64(seed);
        rng.gen_range(min..=max)
    }

    /// Generate a randomized value once during every cycle of `duration`. The
    /// function is completely deterministic given the same parameters in
    /// relation to the current beat. The `seed` - which serves as the root of
    /// an internal seed generator - is also a unique ID for internal slew state
    /// and for that reason you should make sure all animations in your sketch
    /// have unique seeds (unless you want identical animations of course).
    /// `slew` controls smoothing when the value changes with 0.0 being instant
    /// and 1.0 being essentially frozen.
    pub fn random_slewed(
        &self,
        duration: f32,
        (min, max): (f32, f32),
        slew: f32,
        delay: f32,
        stem: u64,
    ) -> f32 {
        let beats = self.beats() - delay;
        let loop_count = ternary!(beats < 0.0, 0.0, (beats / duration).floor());
        let seed = stem + ((duration + (max - min) + loop_count) as u64);
        let mut rng = StdRng::seed_from_u64(seed);
        let value = rng.gen_range(min..=max);

        // Ensures two different calls that share the same seed but differ in
        // delay have the same overall pattern
        let key = stem + (delay.to_bits() as u64 * 10_000_000);

        let mut prev_values = self.random_smooth_previous_values.borrow_mut();
        let value = prev_values.get(&key).map_or(value, |prev| {
            SlewLimiter::slew_pure(*prev, value, slew, slew)
        });

        prev_values.insert(key, value);

        value
    }

    /// Creates a new [`Trigger`] with specified interval and delay;
    /// Use with [`Self::should_trigger`].
    pub fn create_trigger(&self, every: f32, delay: f32) -> Trigger {
        if delay >= every {
            panic!("Delay must be less than interval length");
        }

        Trigger {
            every,
            delay,
            last_trigger_count: -1.0,
        }
    }

    /// Checks if a trigger should fire based on current beat position.
    /// When used with [`Self::create_trigger`], provides a means
    /// of executing arbitrary code at specific intervals
    ///
    /// ```rust
    /// // Do something once every 4 bars
    /// if animation.should_trigger(animation.create_trigger(16.0, 0.0)) {
    ///   // do stuff
    /// }
    /// ```
    pub fn should_trigger(&self, trigger: &mut Trigger) -> bool {
        let total_beats = self.beats();
        let current_interval = (total_beats / trigger.every).floor();
        let position_in_interval = total_beats % trigger.every;

        let should_trigger = current_interval != trigger.last_trigger_count
            && position_in_interval >= trigger.delay;

        if should_trigger {
            trigger.last_trigger_count = current_interval;
        }

        should_trigger
    }
}

#[cfg(test)]
pub mod animation_tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Once;

    // this way each 1/16 = 1 frame, 4 frames per beat,
    // less likely to deal with precision issues.
    pub const FPS: f32 = 24.0;
    pub const BPM: f32 = 360.0;

    static INIT: Once = Once::new();

    pub fn init(frame_count: u32) {
        INIT.call_once(|| {
            env_logger::builder().is_test(true).init();
            frame_controller::ensure_controller(FPS);
        });
        frame_controller::set_frame_count(frame_count);
    }

    pub fn create_instance() -> Animation<FrameTiming> {
        Animation::new(FrameTiming::new(Bpm::new(BPM)))
    }

    #[test]
    #[serial]
    fn test_tri() {
        init(0);
        let a = create_instance();

        let val = a.tri(2.0);
        assert_eq!(val, 0.0, "1/16");

        init(1);
        let val = a.tri(2.0);
        assert_eq!(val, 0.25, "2/16");

        init(2);
        let val = a.tri(2.0);
        assert_eq!(val, 0.5, "3/16");

        init(3);
        let val = a.tri(2.0);
        assert_eq!(val, 0.75, "4/16");

        init(4);
        let val = a.tri(2.0);
        assert_eq!(val, 1.0, "5/16");

        init(5);
        let val = a.tri(2.0);
        assert_eq!(val, 0.75, "6/16");

        init(6);
        let val = a.tri(2.0);
        assert_eq!(val, 0.5, "7/16");

        init(7);
        let val = a.tri(2.0);
        assert_eq!(val, 0.25, "8/16");

        init(8);
        let val = a.tri(2.0);
        assert_eq!(val, 0.0, "9/16");
    }

    #[test]
    #[serial]
    fn test_triangle_8beats_positive_offset() {
        init(0);
        let a = create_instance();

        let val = a.triangle(4.0, (-1.0, 1.0), 0.125);
        assert_eq!(val, -0.75, "1st beat");

        init(15);
        let val = a.triangle(4.0, (-1.0, 1.0), 0.125);
        assert_eq!(val, -1.0, "last beat");

        init(16);
        let val = a.triangle(4.0, (-1.0, 1.0), 0.125);
        assert_eq!(val, -0.75, "1st beat - 2nd cycle");
    }

    #[test]
    #[serial]
    fn test_trigger_on_beat() {
        init(0);
        let animation = create_instance();
        let mut trigger = animation.create_trigger(1.0, 0.0);

        assert!(
            animation.should_trigger(&mut trigger),
            "should trigger at start"
        );

        init(1);
        assert!(
            !animation.should_trigger(&mut trigger),
            "should not trigger mid-beat"
        );

        init(4);
        assert!(
            animation.should_trigger(&mut trigger),
            "should trigger at next beat"
        );
    }

    #[test]
    #[serial]
    fn test_trigger_with_delay() {
        init(0);
        let animation = create_instance();
        let mut trigger = animation.create_trigger(2.0, 0.5);

        assert!(
            !animation.should_trigger(&mut trigger),
            "should not trigger at start due to delay"
        );

        init(2);
        assert!(
            animation.should_trigger(&mut trigger),
            "should trigger at delay point"
        );

        init(4);
        assert!(
            !animation.should_trigger(&mut trigger),
            "should not trigger before next interval"
        );

        init(10);
        assert!(
            animation.should_trigger(&mut trigger),
            "should trigger at next interval after delay"
        );
    }

    #[test]
    #[serial]
    fn test_random() {
        let a = create_instance();
        let r = || a.random(1.0, (0.0, 1.0), 0.0, 999);

        init(0);
        let n = r();

        init(1);
        let n2 = r();
        assert_eq!(n, n2, "should return same N for full cycle");

        init(2);
        let n3 = r();
        assert_eq!(n, n3, "should return same N for full cycle");

        init(3);
        let n4 = r();
        assert_eq!(n, n4, "should return same N for full cycle");

        init(4);
        let n5 = r();
        assert_ne!(n, n5, "should return new number on next cycle");
    }

    #[test]
    #[serial]
    fn test_random_with_delay() {
        let a = create_instance();
        let r = || a.random(1.0, (0.0, 1.0), 0.5, 999);

        init(0);
        let n = r();

        init(4);
        let n2 = r();
        assert_eq!(n, n2, "should return same N for full cycle");

        init(6);
        let n3 = r();
        assert_ne!(n, n3, "should return new number on 2nd cycle");
        init(9);
        let n4 = r();
        assert_eq!(n3, n4, "should stay within 2nd cycle");

        init(10);
        let n5 = r();
        assert_ne!(n4, n5, "should return new number on 3rd cycle");
    }

    #[test]
    #[serial]
    fn test_random_stem() {
        let a = create_instance();
        let r = |stem: u64| a.random(1.0, (0.0, 1.0), 0.0, stem);

        init(0);
        let n1 = r(99);
        let n2 = r(99);

        assert_eq!(n1, n2, "should return same N for same args");

        let n3 = r(100);
        assert_ne!(n1, n3, "should return different N for diff stems");
    }

    #[test]
    #[serial]
    fn test_random_smooth() {
        let a = create_instance();
        let r = || a.random_slewed(1.0, (0.0, 1.0), 0.0, 0.0, 9);

        init(0);
        let n = r();

        init(1);
        let n2 = r();
        assert_eq!(n, n2, "should return same N for full cycle");

        init(2);
        let n3 = r();
        assert_eq!(n, n3, "should return same N for full cycle");

        init(3);
        let n4 = r();
        assert_eq!(n, n4, "should return same N for full cycle");

        init(4);
        let n5 = r();
        assert_ne!(n, n5, "should return new number on next cycle");
    }

    #[test]
    #[serial]
    fn test_random_smooth_with_delay() {
        let a = create_instance();
        let r = || a.random_slewed(1.0, (0.0, 1.0), 0.0, 0.5, 999);

        init(0);
        let n = r();

        init(4);
        let n2 = r();
        assert_eq!(n, n2, "should return same N for full cycle");

        init(6);
        let n3 = r();
        assert_ne!(n, n3, "should return new number on 2nd cycle");
        init(9);
        let n4 = r();
        assert_eq!(n3, n4, "should stay within 2nd cycle");

        init(10);
        let n5 = r();
        assert_ne!(n4, n5, "should return new number on 3rd cycle");
    }
}
