use nannou::color::*;
use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "bug_repro",
    display_name: "Bug Repro",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
};

#[derive(SketchComponents)]
pub struct BugRepro {
    hub: ControlHub<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> BugRepro {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "./bug_repro.yaml"),
        Timing::new(ctx.bpm()),
    );

    BugRepro { hub }
}

impl Sketch for BugRepro {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.4);

        draw.ellipse()
            .color(hsl(0.5, 0.5, 0.5))
            .radius(self.hub.get("radius"))
            .x_y(self.hub.get("x_pos"), 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
