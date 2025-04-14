use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "animation_dev",
    display_name: "Animation Test",
    fps: 60.0,
    bpm: 134.0,
    // fps: 24.0,
    // bpm: 360.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct AnimationDev {
    animation: Animation<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> AnimationDev {
    let animation = Animation::new(Timing::new(ctx.bpm()));

    AnimationDev { animation }
}

impl Sketch for AnimationDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();

        let draw = app.draw();

        draw.rect().x_y(0.0, 0.0).w_h(wr.w(), wr.h()).color(BEIGE);

        let hw = wr.w() / 2.0;
        let hh = wr.h() / 2.0;
        let radius = hh / 5.0;
        let edge = hw - radius;
        let component_value = PHI_F32 - 1.0;

        // RED BALL
        draw.ellipse()
            .x_y(self.animation.triangle(4.0, (-edge, edge), 0.0), hh / 2.0)
            .radius(radius)
            .color(rgb(component_value, 0.0, 0.0));

        // RED RING
        // This should be identical to the above in movement
        draw.ellipse()
            .x_y(
                map_range(
                    self.animation.automate(
                        &[
                            Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                            Breakpoint::ramp(2.0, 1.0, Easing::Linear),
                            Breakpoint::end(4.0, 0.0),
                        ],
                        Mode::Loop,
                    ),
                    0.0,
                    1.0,
                    -edge,
                    edge,
                ),
                hh / 2.0,
            )
            .radius(radius * 1.25)
            .no_fill()
            .stroke_weight(2.0)
            .stroke(rgb(component_value, 0.0, 0.0));

        // YELLOW BALL
        // should match the 1st and 3rd quarters of the above cycle
        draw.ellipse()
            .x_y(
                map_range(
                    self.animation.automate(
                        &[
                            Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                            Breakpoint::step(1.0, 0.5),
                            Breakpoint::ramp(1.5, 0.5, Easing::Linear),
                            Breakpoint::ramp(2.0, 1.0, Easing::Linear),
                            Breakpoint::step(3.0, 0.5),
                            Breakpoint::ramp(3.5, 0.5, Easing::Linear),
                            Breakpoint::end(4.0, 0.0),
                        ],
                        Mode::Loop,
                    ),
                    0.0,
                    1.0,
                    -edge,
                    edge,
                ),
                hh / 4.0,
            )
            .radius(radius * 0.333)
            .color(rgb(component_value, component_value, 0.0));

        // GREEN BALL
        draw.ellipse()
            .x_y(
                map_range(
                    self.animation.loop_phase(8.0),
                    0.0,
                    1.0,
                    -edge,
                    edge,
                ),
                0.0,
            )
            .radius(radius)
            .color(rgb(0.0, component_value, 0.0));

        // TURQUOISE BALL
        let random_freq = 1.0;
        let random_amp = 0.125;
        draw.ellipse()
            .x_y(
                map_range(
                    self.animation.automate(
                        &[
                            Breakpoint::random_smooth(
                                0.0,
                                0.0,
                                random_freq,
                                random_amp,
                                Easing::Linear,
                                Constrain::Clamp(0.0, 1.0),
                            ),
                            Breakpoint::random_smooth(
                                2.0,
                                1.0,
                                random_freq,
                                random_amp,
                                Easing::Linear,
                                Constrain::Clamp(0.0, 1.0),
                            ),
                            Breakpoint::end(4.0, 0.0),
                        ],
                        Mode::Loop,
                    ),
                    0.0,
                    1.0,
                    -edge,
                    edge,
                ),
                -hh / 4.0,
            )
            .radius(radius * 0.333)
            .color(rgb(0.0, component_value, component_value));

        // Testing syncopated delay
        {
            // BLUE BALL
            draw.ellipse()
                .x_y(
                    self.animation.random(2.0, (-edge, edge), 0.0, 9),
                    -hh / 2.0,
                )
                .radius(radius)
                .color(rgb(0.0, 0.0, component_value));

            // DARK TURQUOISE BALL
            draw.ellipse()
                .x_y(
                    self.animation.random(
                        2.0,
                        (-edge, edge),
                        1.0, // delay by 1 beat
                        9,   // same seed for easier comparison
                    ),
                    -hh + hh / 8.0,
                )
                .radius(radius)
                .color(rgb(0.0, 1.0 - component_value, 1.0 - component_value));
        }
        // Testing syncopated delay (smooth version - should follow the above)
        {
            // GRAY RING
            draw.ellipse()
                .x_y(
                    self.animation.random_slewed(
                        2.0,
                        (-edge, edge),
                        0.6,
                        0.0,
                        9,
                    ),
                    -hh / 2.0,
                )
                .radius(radius * 0.666)
                .no_fill()
                .stroke_weight(4.0)
                .stroke(GRAY);

            // LIGHT-GRAY RING
            draw.ellipse()
                .x_y(
                    self.animation.random_slewed(
                        2.0,
                        (-edge, edge),
                        0.6,
                        1.0, // delay by 1 beat
                        9,   // same seed for easier comparison
                    ),
                    -hh + hh / 8.0,
                )
                .radius(radius * 0.666)
                .no_fill()
                .stroke_weight(4.0)
                .stroke(LIGHTGRAY);
        }

        {
            // BLACK BALL LEFT
            draw.ellipse()
                .x_y(
                    -wr.qw(),
                    self.animation.random(1.0, (-wr.hh(), wr.hh()), 0.0, 999),
                )
                .radius(20.0)
                .color(BLACK);

            // BLACK BALL RIGHT
            draw.ellipse()
                .x_y(
                    wr.qw(),
                    self.animation.random_slewed(
                        1.0,
                        (-wr.hh(), wr.hh()),
                        0.7,
                        0.0,
                        99,
                    ),
                )
                .radius(20.0)
                .color(BLACK);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
