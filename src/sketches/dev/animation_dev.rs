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
    lerp: f32,
    ramp: f32,
    r_ramp: f32,
    random_anim: f32,
    slew_limiter: SlewLimiter,
}

pub fn init(_app: &App, ctx: LatticeContext) -> AnimationDev {
    let animation = Animation::new(Timing::new(ctx.bpm));

    AnimationDev {
        animation,
        lerp: 0.0,
        ramp: 0.0,
        r_ramp: 0.0,
        random_anim: 0.0,
        slew_limiter: SlewLimiter::default(),
    }
}

impl Sketch for AnimationDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.lerp = self
            .animation
            .lerp(&[kf(0.0, 2.0), kf(1.0, 2.0), kf(0.0, 0.0)], 0.0);

        self.ramp = self.animation.ramp(
            &[kf(0.0, 4.0), kf(1.0, 4.0)],
            0.0,
            1.0,
            Easing::Linear,
        );

        self.r_ramp = self.animation.r_ramp(
            &[kfr((0.0, 1.0), 4.0)],
            0.0,
            1.0,
            Easing::Linear,
        );

        let random_anim = self.animation.automate(
            &[
                Breakpoint::random(0.0, 0.5, 0.5),
                Breakpoint::random(2.0, 0.5, 0.5),
            ],
            Mode::Loop,
        );
        self.random_anim =
            self.slew_limiter.slew_with_rates(random_anim, 0.8, 0.8);
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
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
            .x_y(map_range(self.lerp, 0.0, 1.0, -edge, edge), hh / 2.0)
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
            .x_y(map_range(self.ramp, 0.0, 1.0, -edge, edge), 0.0)
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

        // BLUE BALL
        draw.ellipse()
            .x_y(map_range(self.r_ramp, 0.0, 1.0, -edge, edge), -hh / 2.0)
            .radius(radius)
            .color(rgb(0.0, 0.0, component_value));

        // DARK TURQUOISE BALL
        draw.ellipse()
            .x_y(
                map_range(self.random_anim, 0.0, 1.0, -edge, edge),
                -hh + hh / 8.0,
            )
            .radius(radius * 0.333)
            .color(rgb(0.0, 1.0 - component_value, 1.0 - component_value));

        draw.to_frame(app, &frame).unwrap();
    }
}
