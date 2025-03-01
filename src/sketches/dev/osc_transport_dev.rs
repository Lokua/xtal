use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "osc_transport_test",
    display_name: "OSC Transport Test",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(LegacySketchComponents)]
pub struct Model {
    animation: Animation<OscTransportTiming>,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(OscTransportTiming::new(SKETCH_CONFIG.bpm));

    Model { animation, wr }
}

pub fn update(_app: &App, _m: &mut Model, _update: Update) {}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .color(BLACK)
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h());

    let a = m.animation.lrp(&[kf(0.0, 4.0), kf(200.0, 4.0)], 0.0);
    let b = m.animation.lrp(&[kf(0.0, 2.0), kf(200.0, 2.0)], 0.0);

    draw.ellipse()
        .color(rgba(1.0, 0.0, 0.0, 0.5))
        .radius(a)
        .x_y(-m.wr.w() / 16.0, 0.0);

    draw.ellipse()
        .color(rgba(0.0, 0.0, 1.0, 0.5))
        .radius(b)
        .x_y(m.wr.w() / 16.0, 0.0);

    draw.to_frame(app, &frame).unwrap();
}
