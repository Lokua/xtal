use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "control_script_test",
    display_name: "ControlScript Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

#[derive(SketchComponents)]
pub struct Model {
    controls: ControlScript,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(to_absolute_path(
        file!(),
        "control_script_test.yaml",
    ));

    Model { controls, wr }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.controls.update();
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    // background
    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .hsla(0.0, 0.0, 0.02, 0.1);

    let hue = m.controls.get("hue");
    let radius = m.controls.get("radius");
    let pos_x = m.controls.get("/pos_x");

    draw.ellipse()
        .color(hsl(hue, 0.5, 0.5))
        .radius(radius)
        .x_y(0.0, 0.0);

    draw.ellipse()
        .color(WHITE)
        .radius(20.0)
        .x_y(pos_x * m.wr.hw(), 0.0);

    draw.to_frame(app, &frame).unwrap();
}
