use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "osc_dev",
    display_name: "OSC Test",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct OscDev {
    osc: OscControls,
}

pub fn init(_app: &App, _ctx: &Ctx) -> OscDev {
    let osc = OscControlBuilder::new()
        .control("a", (0.0, 400.0), 0.5)
        .control("b", (0.0, 400.0), 0.5)
        .build();

    OscDev { osc }
}

impl Sketch for OscDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Ctx) {
        debug_throttled!(
            1_000,
            "a: {}, /b: {}",
            self.osc.get("a"),
            self.osc.get("b")
        );
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Ctx) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().color(BLACK).x_y(0.0, 0.0).w_h(wr.w(), wr.h());

        let a = self.osc.get("a");
        let b = self.osc.get("b");

        draw.ellipse()
            .color(rgba(1.0, 0.0, 0.0, 0.5))
            .radius(a)
            .x_y(-wr.w() / 16.0, 0.0);

        draw.ellipse()
            .color(rgba(0.0, 0.0, 1.0, 0.5))
            .radius(b)
            .x_y(wr.w() / 16.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
