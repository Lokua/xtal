use super::prelude::*;

// Split implementation file for Animation struct since that file is getting a
// bit large.

#[derive(Debug)]
pub struct Breakpoint {
    pub kind: Kind,
    pub position: f32,
    pub value: f32,
}

impl Breakpoint {
    /// Create a step that will be held at `value` until the next breakpoint.
    pub fn step(position: f32, value: f32) -> Self {
        Self::new(Kind::Step, position, value)
    }

    /// Create a step that will curve from this `value` to the next breakpoint's
    /// value with adjustable easing.
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

    fn new(kind: Kind, position: f32, value: f32) -> Self {
        Self {
            kind,
            position,
            value,
        }
    }
}

#[derive(Debug)]
pub enum Kind {
    Step,
    Ramp {
        easing: Easing,
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

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Sine,
    Triangle,
    Square,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Constrain {
    None,
    Clamp(f32, f32),
    Fold(f32, f32),
    Wrap(f32, f32),
}

impl Constrain {
    pub fn from_str(method: &str, min: f32, max: f32) -> Self {
        match method.to_lowercase().as_str() {
            "none" => Self::None,
            "clamp" => Self::Clamp(min, max),
            "fold" => Self::Fold(min, max),
            "wrap" => Self::Wrap(min, max),
            _ => loud_panic!("No constrain method {} exists.", method),
        }
    }

    pub fn apply(&self, value: f32) -> f32 {
        match self {
            Self::None => value,
            Self::Clamp(min, max) => constrain::clamp(value, *min, *max),
            Self::Fold(min, max) => constrain::fold(value, *min, *max),
            Self::Wrap(min, max) => constrain::wrap(value, *min, *max),
        }
    }
}

impl Shape {
    pub fn from_str(shape: &str) -> Shape {
        match shape.to_lowercase().as_str() {
            "sine" => Shape::Sine,
            "triangle" => Shape::Triangle,
            "square" => Shape::Square,
            _ => loud_panic!("No shape {} exists.", shape),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Loop,
    Once,
}

impl<T: TimingSource> Animation<T> {
    /// An advanced animation method modelled on DAW automation lanes.
    /// See src/sketches/scratch/breakpoints_vis.rs for a demonstration
    pub fn animate(&self, breakpoints: &[Breakpoint], mode: Mode) -> f32 {
        assert!(breakpoints.len() >= 1, "At least 1 breakpoint is required");
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
                    ramp(p1, p2, beats_elapsed, easing.clone())
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
                        let value = ramp(p1, p2, beats_elapsed, easing.clone());

                        let phase_in_cycle = beats_elapsed / frequency;

                        let t = phase_in_cycle % 1.0;
                        let m = 2.0 * (width - 0.5);
                        let mod_wave =
                            ((TWO_PI * t) + m * (TWO_PI * t).sin()).sin();

                        constrain.apply(value + (mod_wave * amplitude))
                    }
                    Shape::Triangle => {
                        let value = ramp(p1, p2, beats_elapsed, easing.clone());

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
                        let value = ramp(p1, p2, beats_elapsed, easing.clone());
                        let phase_in_cycle = beats_elapsed / frequency;

                        let mod_wave = if (phase_in_cycle % 1.0) < *width {
                            1.0
                        } else {
                            -1.0
                        };

                        constrain.apply(value + (mod_wave * amplitude))
                    }
                },
                Kind::End => {
                    loud_panic!("Somehow we've moved beyond the end")
                }
            },
            _ => {
                warn!("Could not match any breakpoints");
                0.0
            }
        }
    }
}

fn ramp(
    p1: &Breakpoint,
    p2: &Breakpoint,
    beats_elapsed: f32,
    easing: Easing,
) -> f32 {
    let duration = p2.position - p1.position;
    let t = easing.apply((beats_elapsed / duration) % 1.0);
    let value = lerp(p1.value, p2.value, t);
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // 1 frame = 1/16; 4 frames per beat; 16 frames per bar
    use crate::framework::animation::tests::{create_instance, init};

    #[test]
    #[serial]
    fn test_breakpoint_step_init() {
        init(0);
        let a = create_instance();
        let x = a.animate(&[Breakpoint::step(0.0, 44.0)], Mode::Once);
        assert_eq!(x, 44.0, "Returns initial value");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_2nd() {
        init(4);
        let a = create_instance();
        let x = a.animate(
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
        let x = a.animate(
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
        let x = a.animate(breakpoints, Mode::Loop);
        assert_eq!(x, 20.0, "Returns 2nd stage");
        init(8);
        let x = a.animate(breakpoints, Mode::Loop);
        assert_eq!(x, 10.0, "Returns 1st stage when looping back around");
    }

    #[test]
    #[serial]
    fn test_breakpoint_step_midway() {
        init(2);
        let a = create_instance();
        let x = a.animate(
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
        let x = a.animate(
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
        let x = a.animate(
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
            a.animate(
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
            a.animate(
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
            a.animate(
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
}
