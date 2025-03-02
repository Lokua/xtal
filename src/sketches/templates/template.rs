use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "template",
    display_name: "Template",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

pub struct Template {
    animation: Animation<Timing>,
    controls: Controls,
    radius: f32,
    hue: f32,
}

pub fn init(_app: &App, ctx: LatticeContext) -> Template {
    debug!("ctx: {:?}", ctx);
    let animation = Animation::new(Timing::new(ctx.bpm));

    let controls = Controls::new(vec![Control::slider(
        "radius",
        100.0,
        (10.0, 500.0),
        1.0,
    )]);

    Template {
        animation,
        controls,
        radius: 0.0,
        hue: 0.0,
    }
}

impl SketchDerived for Template {}

impl Sketch for Template {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        let radius_max = self.controls.float("radius");

        self.radius = self.animation.automate(
            &[
                Breakpoint::ramp(0.0, 10.0, Easing::Linear),
                Breakpoint::ramp(0.5, ctx.window_rect().hw(), Easing::Linear),
                Breakpoint::ramp(0.1, 10.0, Easing::Linear),
                Breakpoint::ramp(1.5, radius_max, Easing::Linear),
                Breakpoint::end(2.0, 10.0),
            ],
            Mode::Loop,
        );

        self.hue = self.animation.tri(12.0)
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.4);

        draw.ellipse()
            .color(hsl(self.hue, 0.5, 0.5))
            .radius(self.radius)
            .x_y(0.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
