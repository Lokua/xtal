use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "cv_test",
    display_name: "CV Test",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct Model {
    cv: CvControls,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let cv = CvControlBuilder::new(SKETCH_CONFIG.fps)
        .control_mapped("a", 0, (0.0, 400.0), 0.5)
        .control_mapped("b", 1, (0.0, 400.0), 0.5)
        .build();

    Model { cv, wr }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    debug_throttled!(1_000, "a: {}, b: {}", m.cv.get("a"), m.cv.get("b"));
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .color(BLACK)
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h());

    let a = m.cv.get("a");
    let b = m.cv.get("b");

    draw.ellipse()
        .color(rgba(1.0, 0.0, 0.0, 0.5))
        .radius(a)
        .x_y(-m.wr.w_(16.0), 0.0);

    draw.ellipse()
        .color(rgba(0.0, 0.0, 1.0, 0.5))
        .radius(b)
        .x_y(m.wr.w_(16.0), 0.0);

    draw.to_frame(app, &frame).unwrap();
}
