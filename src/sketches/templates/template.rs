use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "template",
    display_name: "Template",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(200),
};

#[derive(SketchComponents)]
pub struct Template {
    hub: ControlHub<Timing>,
    hue: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Template {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("radius", 100.0, (10.0, 300.0), 1.0, None)
        .build();

    Template { hub, hue: 0.0 }
}

impl Sketch for Template {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.hue = self.hub.animation.tri(12.0)
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
            .radius(self.hub.get("radius"))
            .x_y(0.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
