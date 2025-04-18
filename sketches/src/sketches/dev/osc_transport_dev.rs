use nannou::prelude::*;

use lattice::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "osc_transport_test",
    display_name: "OSC Transport Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct OscTransportDev {
    animation: Animation<OscTransportTiming>,
}

pub fn init(_app: &App, ctx: &Context) -> OscTransportDev {
    let animation = Animation::new(OscTransportTiming::new(ctx.bpm()));

    OscTransportDev { animation }
}

impl Sketch for OscTransportDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().color(BLACK).x_y(0.0, 0.0).w_h(wr.w(), wr.h());

        let a = self.animation.triangle(8.0, (0.0, 200.0), 0.0);
        let b = self.animation.triangle(4.0, (0.0, 200.0), 0.0);

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
