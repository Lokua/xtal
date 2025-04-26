//! Animation module providing musically-timed animation and transition methods

use nannou::math::map_range;
use nannou::rand::rngs::StdRng;
use nannou::rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::str::FromStr;

use crate::framework::frame_controller;
use crate::framework::prelude::*;

/// Data structure used in conjunction with
/// [`Animation::create_trigger`] and [`Animation::should_trigger`]
#[derive(Debug)]
pub struct Trigger {
    every: f32,
    delay: f32,
    last_trigger_count: f32,
}

/// The core structure needed to configure segments for the
/// [`Animation::automate`] method. See the various constructors such as
/// [`Breakpoint::step`], [`Breakpoint::ramp`], etc. for in depth details.
#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub kind: Kind,
    pub position: f32,
    pub value: f32,
}

impl Breakpoint {
    pub fn new(kind: Kind, position: f32, value: f32) -> Self {
        Self {
            kind,
            position,
            value,
        }
    }

    /// Create a step that will be held at `value` until the next breakpoint.
    pub fn step(position: f32, value: f32) -> Self {
        Self::new(Kind::Step, position, value)
    }

    /// Create a step that will curve from this `value` to the next breakpoint's
    /// value with adjustable easing. `position` is expressed in beats.
    pub fn ramp(position: f32, value: f32, easing: Easing) -> Self {
        Self::new(Kind::Ramp { easing }, position, value)
    }

    /// Creates a linear ramp from this `value` to the next breakpoint's value
    /// with amplitude modulation applied over it and finalized by various
    /// clamping modes and easing algorithms that together can produce extremely
    /// complex curves. Like position, `frequency` is expressed in beats.
    /// `amplitude` represents how much above and below the base interpolated
    /// value the modulation will add or subtract depending on its phase.
    /// Negative amplitudes can be used to invert the modulation. For
    /// [`Shape::Sine`] and [`Shape::Triangle`], the modulation wave is phase
    /// shifted to always start and end at or very close to zero to ensure
    /// smooth transitions between segments (this is not the case for
    /// [`Shape::Square`] because discontinuities are unavoidable). The `width`
    /// parameter controls the [`Shape::Square`] duty cycle. For `Sine` and
    /// `Triangle` shapes, it will skew the peaks, for example when applied to a
    /// triangle a `width` of 0.0 will produce a downwards saw while 1.0 will
    /// produce an upwards one - applied to sine is a similarly skewed,
    /// asymmetric wave. For all shapes a value of 0.5 will produce the natural
    /// wave. Beware this method can produce values outside of the otherwise
    /// normalized \[0, 1\] range when the `constrain` parameter is set to
    /// [`Constrain::None`].
    #[allow(clippy::too_many_arguments)]
    pub fn wave(
        position: f32,
        value: f32,
        shape: Shape,
        frequency: f32,
        width: f32,
        amplitude: f32,
        easing: Easing,
        constrain: Constrain,
    ) -> Self {
        Self::new(
            Kind::Wave {
                shape,
                frequency,
                width,
                amplitude,
                easing,
                constrain,
            },
            position,
            value,
        )
    }

    /// Create a step chosen randomly from the passed in `amplitude` which
    /// specifies the range of possible deviation from `value`.
    ///
    /// > TIP: you can make this a smooth random by applying a
    /// > [`SlewLimiter`] to the output.
    pub fn random(position: f32, value: f32, amplitude: f32) -> Self {
        Self::new(Kind::Random { amplitude }, position, value)
    }

    /// # ⚠️ Experimental
    /// Similar to [`Self::wave`], only uses Perlin noise to amplitude modulate
    /// the base curve. Useful for adding jitter when `frequency` is shorter
    /// than the duration of this point's `position` and the next; larger values
    /// equal to that duration or longer are much smoother.
    pub fn random_smooth(
        position: f32,
        value: f32,
        frequency: f32,
        amplitude: f32,
        easing: Easing,
        constrain: Constrain,
    ) -> Self {
        Self::new(
            Kind::RandomSmooth {
                frequency,
                amplitude,
                easing,
                constrain,
            },
            position,
            value,
        )
    }

    /// The last breakpoint in any sequence represents the final value and is
    /// never actually entered. Technically any kind of breakpoint can be used
    /// at the end and will be interpreted exactly the same way (only value and
    /// position will be used to mark the end of a sequence), but this is
    /// provided for code clarity as it reads better. If you are using
    /// [`Mode::Loop`] it's a good idea to make the value of this endpoint match
    /// the first value to avoid discontinuity (unless you want that).
    pub fn end(position: f32, value: f32) -> Self {
        Self::new(Kind::End, position, value)
    }
}

#[derive(Clone, Debug)]
pub enum Kind {
    Step,
    Ramp {
        easing: Easing,
    },
    Random {
        amplitude: f32,
    },
    RandomSmooth {
        frequency: f32,
        amplitude: f32,
        easing: Easing,
        constrain: Constrain,
    },
    Wave {
        shape: Shape,
        amplitude: f32,
        width: f32,
        frequency: f32,
        easing: Easing,
        constrain: Constrain,
    },
    End,
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "step" => Ok(Kind::Step),
            "ramp" => Ok(Kind::Ramp {
                easing: Easing::Linear,
            }),
            "random" => Ok(Kind::Random { amplitude: 0.25 }),
            "randomsmooth" => Ok(Kind::RandomSmooth {
                frequency: 0.25,
                amplitude: 0.25,
                easing: Easing::Linear,
                constrain: Constrain::None,
            }),
            "wave" => Ok(Kind::Wave {
                shape: Shape::Sine,
                frequency: 0.25,
                width: 0.5,
                amplitude: 0.25,
                easing: Easing::Linear,
                constrain: Constrain::None,
            }),
            "end" => Ok(Kind::End),
            _ => Err(format!("Unknown breakpoint kind variant: {}", s)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Sine,
    Triangle,
    Square,
}
impl FromStr for Shape {
    type Err = String;

    fn from_str(shape: &str) -> Result<Self, Self::Err> {
        match shape.to_lowercase().as_str() {
            "sine" => Ok(Shape::Sine),
            "triangle" => Ok(Shape::Triangle),
            "square" => Ok(Shape::Square),
            _ => Err(format!("No shape {} exists.", shape)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Loop,
    Once,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode.to_lowercase().as_str() {
            "loop" => Ok(Mode::Loop),
            "once" => Ok(Mode::Once),
            _ => Err(format!("No mode {} exists.", mode)),
        }
    }
}

///  Animation module providing musically-timed animation methods with support
///  for incredibly easy to use basic oscillations as well as ultra-complex and
///  expressive automation
///
///  # Basic Usage
///
///  ```rust
///  let animation = Animation::new(Timing::new(ctx.bpm()));
///
///  // Simple ramp oscillation from 0.0 to 1.0 over 4 beats (repeating)
///  let phase = animation.loop_phase(4.0);
///
///  // Triangle wave oscillation between ranges
///  let value = animation.triangle(
///      // Duration in beats
///      4.0,
///      // Min/max range
///      (0.0, 100.0),  
///      // Phase offset
///      0.0,           
///  );
///  ```
///
///  # Advanced Automation
///
///  The [`Animation::automate`] method provides DAW-style automation curves
///  with multiple breakpoint types and transition modes:
///
///  ```rust
///  let value = animation.automate(
///      &[
///          // Start with a step change
///          Breakpoint::step(0.0, 0.0),
///          // Ramp with exponential easing
///          Breakpoint::ramp(1.0, 1.0, Easing::EaseInExpo),
///          // Add amplitude modulation
///          Breakpoint::wave(
///              // Position in beats
///              2.0,
///              // Base value
///              0.5,
///              Shape::Sine,
///              // Frequency in beats
///              0.25,
///              // Width
///              0.5,
///              // Amplitude
///              0.25,
///              Easing::Linear,
///              Constrain::None,
///          ),
///          // Mark end of sequence
///          Breakpoint::end(4.0, 0.0),
///      ],
///      Mode::Loop
///  );
///  ```
///
/// See [`crate::prelude::effects`] for ways you can post-process the results
/// of any animation method to achieve more complex results
#[derive(Clone, Debug)]
pub struct Animation<T: TimingSource> {
    pub timing: T,
    random_smooth_previous_values: RefCell<HashMap<u64, f32>>,
}

impl<T: TimingSource> Animation<T> {
    pub fn new(timing: T) -> Self {
        Self {
            timing,
            random_smooth_previous_values: RefCell::new(HashMap::default()),
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
        assert!(delay <= every, "Delay must be less than interval length");

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

    /// An advanced animation method modelled on DAW automation lanes. It is
    /// capable of producing the same results as just about every other
    /// animation method yet is more powerful, but as such requires a bit more
    /// configuration. While other animation methods are focused on one style of
    /// keyframe/transition, `automate` allows many different types of
    /// transitions defined by a list of [`Breakpoint`], each with its own
    /// configurable [`Kind`]. See [breakpoints] for a static visualization of
    /// the kinds of curves `automate` can produce.
    ///
    /// [breakpoints]: https://github.com/Lokua/xtal/blob/main/src/sketches/breakpoints.rs
    pub fn automate(&self, breakpoints: &[Breakpoint], mode: Mode) -> f32 {
        assert!(!breakpoints.is_empty(), "At least 1 breakpoint is required");
        assert!(
            breakpoints[0].position == 0.0,
            "First breakpoint must be 0.0"
        );

        if breakpoints.len() == 1 {
            return breakpoints[0].value;
        }

        let total_beats = breakpoints.last().unwrap().position;

        let beats_elapsed = ternary!(
            mode == Mode::Loop,
            self.beats() % total_beats,
            self.beats()
        );

        let mut breakpoint: Option<&Breakpoint> = None;
        let mut next_point: Option<&Breakpoint> = None;

        for (index, point) in breakpoints.iter().enumerate() {
            if index == breakpoints.len() - 1 && mode != Mode::Loop {
                return point.value;
            }

            let next = &breakpoints[(index + 1) % breakpoints.len()];

            if next.position < point.position {
                breakpoint = Some(next);
                next_point = Some(point);
                break;
            }

            if point.position <= beats_elapsed && next.position > beats_elapsed
            {
                breakpoint = Some(point);
                next_point = Some(next);
                break;
            }
        }

        match (breakpoint, next_point) {
            (Some(p1), None) => p1.value,
            (Some(p1), Some(p2)) => match &p1.kind {
                Kind::Step => p1.value,
                Kind::Ramp { easing } => {
                    Self::create_ramp(p1, p2, beats_elapsed, easing.clone())
                }
                Kind::Wave {
                    shape,
                    frequency,
                    width,
                    amplitude,
                    easing,
                    constrain,
                } => match shape {
                    Shape::Sine => {
                        let value = Self::create_ramp(
                            p1,
                            p2,
                            beats_elapsed,
                            easing.clone(),
                        );

                        let phase_in_cycle = beats_elapsed / frequency;

                        let t = phase_in_cycle % 1.0;
                        let m = 2.0 * (width - 0.5);
                        let mod_wave =
                            ((TWO_PI * t) + m * (TWO_PI * t).sin()).sin();

                        constrain.apply(value + (mod_wave * amplitude))
                    }
                    Shape::Triangle => {
                        let value = Self::create_ramp(
                            p1,
                            p2,
                            beats_elapsed,
                            easing.clone(),
                        );

                        let phase_offset = 0.25;
                        let phase_in_cycle = beats_elapsed / frequency;
                        let mut mod_wave =
                            (phase_in_cycle + phase_offset) % 1.0;

                        mod_wave = if mod_wave < *width {
                            4.0 * mod_wave - 1.0
                        } else {
                            3.0 - 4.0 * mod_wave
                        };

                        constrain.apply(value + (mod_wave * amplitude))
                    }
                    Shape::Square => {
                        let value = Self::create_ramp(
                            p1,
                            p2,
                            beats_elapsed,
                            easing.clone(),
                        );
                        let phase_in_cycle = beats_elapsed / frequency;

                        let mod_wave = if (phase_in_cycle % 1.0) < *width {
                            1.0
                        } else {
                            -1.0
                        };

                        constrain.apply(value + (mod_wave * amplitude))
                    }
                },
                Kind::Random { amplitude } => {
                    let loop_count = (self.beats() / p2.position).floor();
                    let seed = (p1.position
                        + p2.position
                        + p1.value
                        + amplitude
                        + loop_count) as u64;
                    let mut rng = StdRng::seed_from_u64(seed);
                    let y = p1.value;
                    rng.gen_range(y - amplitude..=y + amplitude)
                }
                Kind::RandomSmooth {
                    frequency,
                    amplitude,
                    easing,
                    constrain,
                } => {
                    let value = Self::create_ramp(
                        p1,
                        p2,
                        beats_elapsed,
                        easing.clone(),
                    );

                    let x = (beats_elapsed / frequency) % 1.0;
                    let y = value;
                    let loop_count = (self.beats() / p2.position).floor();
                    let seed = (p1.position
                        + p2.position
                        + p1.value
                        + amplitude
                        + loop_count) as u64;
                    let noise_scale = 2.5;
                    let random_value = PerlinNoise::new(seed as u32)
                        .get([x * noise_scale, y * noise_scale]);

                    let random_mapped = map_range(
                        random_value,
                        -1.0,
                        1.0,
                        -amplitude,
                        amplitude + f32::EPSILON,
                    );

                    constrain.apply(value + random_mapped)
                }
                Kind::End => {
                    panic!("Somehow we've moved beyond the end")
                }
            },
            _ => {
                warn_once!("Could not match breakpoint {:?}", breakpoint);
                0.0
            }
        }
    }

    fn create_ramp(
        p1: &Breakpoint,
        p2: &Breakpoint,
        beats_elapsed: f32,
        easing: Easing,
    ) -> f32 {
        let duration = p2.position - p1.position;
        let t = easing.apply(((beats_elapsed - p1.position) / duration) % 1.0);
        lerp(p1.value, p2.value, t)
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
            frame_controller::set_fps(FPS);
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

    #[test]
    #[serial]
    fn test_breakpoint_step_init() {
        init(0);
        let a = create_instance();
        let x = a.automate(&[Breakpoint::step(0.0, 44.0)], Mode::Once);
        assert_eq!(x, 44.0, "Returns initial value");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_2nd() {
        init(4);
        let a = create_instance();
        let x = a.automate(
            &[Breakpoint::step(0.0, 10.0), Breakpoint::step(1.0, 20.0)],
            Mode::Once,
        );
        assert_eq!(x, 20.0, "Returns 2nd stage");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_last() {
        init(100);
        let a = create_instance();
        let x = a.automate(
            &[Breakpoint::step(0.0, 10.0), Breakpoint::step(1.0, 20.0)],
            Mode::Once,
        );
        assert_eq!(x, 20.0, "Returns last stage");
    }

    #[test]
    #[serial]

    fn test_breakpoint_step_loop_mode() {
        init(4);
        let breakpoints = &[
            Breakpoint::step(0.0, 10.0),
            Breakpoint::step(1.0, 20.0),
            Breakpoint::end(2.0, 0.0),
        ];
        let a = create_instance();
        let x = a.automate(breakpoints, Mode::Loop);
        assert_eq!(x, 20.0, "Returns 2nd stage");
        init(8);
        let x = a.automate(breakpoints, Mode::Loop);
        assert_eq!(x, 10.0, "Returns 1st stage when looping back around");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_midway() {
        init(2);
        let a = create_instance();
        let x = a.automate(
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                Breakpoint::end(1.0, 1.0),
            ],
            Mode::Once,
        );
        assert_eq!(x, 0.5, "Returns midway point");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_last_16th() {
        init(3);
        let a = create_instance();
        let x = a.automate(
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                Breakpoint::end(1.0, 1.0),
            ],
            Mode::Once,
        );
        assert_eq!(x, 0.75, "Returns midway point");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_last_16th_loop() {
        init(7);
        let a = create_instance();
        let x = a.automate(
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                Breakpoint::end(1.0, 1.0),
            ],
            Mode::Loop,
        );
        assert_eq!(x, 0.75, "Returns midway point");
    }

    #[test]
    #[serial]
    fn test_step_then_ramp() {
        let a = create_instance();
        let x = || {
            a.automate(
                &[
                    Breakpoint::step(0.0, 10.0),
                    Breakpoint::ramp(1.0, 20.0, Easing::Linear),
                    Breakpoint::end(2.0, 10.0),
                ],
                Mode::Loop,
            )
        };

        init(0);
        assert_eq!(x(), 10.0);
        init(1);
        assert_eq!(x(), 10.0);
        init(2);
        assert_eq!(x(), 10.0);
        init(3);
        assert_eq!(x(), 10.0);

        init(4);
        assert_eq!(x(), 20.0);
        init(5);
        assert_eq!(x(), 17.5);
        init(6);
        assert_eq!(x(), 15.0);
        init(7);
        assert_eq!(x(), 12.5);

        init(8);
        assert_eq!(x(), 10.0);
    }

    #[test]
    #[serial]
    fn test_wave_triangle_simple() {
        let a = create_instance();
        let x = || {
            a.automate(
                &[
                    Breakpoint::wave(
                        0.0,
                        0.0,
                        Shape::Triangle,
                        1.0,
                        0.5,
                        0.5,
                        Easing::Linear,
                        Constrain::None,
                    ),
                    Breakpoint::end(1.0, 1.0),
                ],
                Mode::Loop,
            )
        };

        // 0 beats
        init(0);
        assert_eq!(x(), 0.0);

        // 0.25 beats, base 0.25 + wave 0.5 = 0.75
        init(1);
        assert_eq!(x(), 0.75);

        // 0.5 beats, base 0.5 + wave 0.0 = 0.5
        init(2);
        assert_eq!(x(), 0.5);

        // 0.75 beats, base 0.75 + wave -0.5 = 0.25
        init(3);
        assert_eq!(x(), 0.25);

        // And back around
        init(4);
        assert_eq!(x(), 0.0);
    }

    #[test]
    #[serial]
    fn test_step_to_ramp_edge_case() {
        let a = create_instance();
        let x = || {
            a.automate(
                &[
                    Breakpoint::step(0.0, 0.0),
                    Breakpoint::step(0.5, 1.0),
                    Breakpoint::ramp(1.0, 0.5, Easing::EaseInExpo),
                    Breakpoint::wave(
                        1.5,
                        1.0,
                        Shape::Triangle,
                        0.25,
                        0.5,
                        0.25,
                        Easing::Linear,
                        Constrain::None,
                    ),
                    Breakpoint::end(2.0, 0.0),
                ],
                Mode::Once,
            )
        };

        init(4);
        assert_eq!(x(), 0.5);
    }

    #[test]
    #[serial]
    fn test_ramp_bug_2025_02_23() {
        let a = create_instance();
        let x = || {
            a.automate(
                &[
                    Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                    Breakpoint::ramp(32.0, 0.5, Easing::Linear),
                    Breakpoint::ramp(96.0, 1.0, Easing::Linear),
                    Breakpoint::ramp(128.0, 0.75, Easing::Linear),
                    Breakpoint::end(192.0, 0.25),
                ],
                Mode::Once,
            )
        };

        init(128);
        assert_eq!(x(), 0.5);
    }
}
