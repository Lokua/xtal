use super::prelude::*;

/// Split implemetation file for Animation::animate,
/// since that file is getting a bit large.

#[derive(Debug)]
pub struct Breakpoint {
    pub kind: Transition,
    pub position: f32,
    pub value: f32,
}

#[derive(Debug)]
pub enum Transition {
    Step,
    Ramp {
        easing: Easing,
    },
    Wave {
        shape: Shape,
        amplitude: f32,
        frequency: f32,
    },
    End,
}

impl Breakpoint {
    pub fn new(kind: Transition, position: f32, value: f32) -> Self {
        Self {
            kind,
            position,
            value,
        }
    }

    pub fn step(position: f32, value: f32) -> Self {
        Self::new(Transition::Step, position, value)
    }

    pub fn ramp(position: f32, value: f32, easing: Easing) -> Self {
        Self::new(Transition::Ramp { easing }, position, value)
    }

    pub fn wave(
        position: f32,
        value: f32,
        shape: Shape,
        frequency: f32,
        amplitude: f32,
    ) -> Self {
        Self::new(
            Transition::Wave {
                shape,
                frequency,
                amplitude,
            },
            position,
            value,
        )
    }

    pub fn end(position: f32, value: f32) -> Self {
        Self::new(Transition::End, position, value)
    }
}

#[derive(Debug, PartialEq)]
pub enum Shape {
    Sine,
    Triangle,
    Saw, // SawUp, SawDown?
    Square,
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Loop,
    Once,
}

impl<T: TimingSource> Animation<T> {
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
            (Some(bp), None) => bp.value,
            (Some(bp), Some(np)) => match &bp.kind {
                Transition::Step => bp.value,
                Transition::Ramp { easing } => {
                    let duration = np.position - bp.position;
                    let t = (beats_elapsed / duration) % 1.0;
                    let value = lerp(bp.value, np.value, t);
                    easing.apply(value)
                }
                Transition::Wave {
                    shape,
                    frequency,
                    amplitude,
                } => match shape {
                    Shape::Sine => unimplemented!(),
                    Shape::Triangle => {
                        // TODO: optimize by skipping math when next step value
                        // is same as the previous?
                        let duration = np.position - bp.position;
                        let t = (beats_elapsed / duration) % 1.0;
                        let value = lerp(bp.value, np.value, t);

                        // So our triangle starts 1/4 cycle in at 0
                        // and start/connect segments smoothly
                        // WIP: I have no idea how to calculate this properly
                        let phase_offset = 0.25;
                        let mut m =
                            ((beats_elapsed / frequency) + phase_offset) % 1.0;
                        m = ternary!(m < 0.5, m, 1.0 - m) * 4.0 - 1.0;
                        m *= amplitude;

                        value + m
                    }
                    Shape::Saw => unimplemented!(),
                    Shape::Square => unimplemented!(),
                },
                Transition::End => {
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
    fn test_wave_triangle() {
        init(0);
        let a = create_instance();
        let x = || {
            a.animate(
                &[
                    Breakpoint::wave(0.0, 0.0, Shape::Triangle, 0.25, 0.25),
                    Breakpoint::end(2.0, 1.0),
                ],
                Mode::Once,
            )
        };

        for frame in 0..=7 {
            init(frame);
            debug!("{}", x());
        }

        init(0);
        assert_eq!(x(), 0.0);

        init(1);
        assert_eq!(x(), 0.125 + 0.25); // 0.375

        init(2);
        assert_eq!(x(), 0.25);

        init(3);
        assert_eq!(x(), 0.375 - 0.25); // 0.125

        init(4);
        assert_eq!(x(), 0.5);

        init(5);
        assert_eq!(x(), 0.625 + 0.25); // 0.875

        init(6);
        assert_eq!(x(), 0.75);

        init(7);
        assert_eq!(x(), 0.875 - 0.25); //0.625
    }
}
