use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "animation_script_test",
    display_name: "Animation Script Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 125.0,
    // bpm: 20.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

#[derive(SketchComponents)]
pub struct Model {
    // animation_script: AnimationScript<MidiSongTiming>,
    animation_script: AnimationScript<MidiSongTiming>,
    controls: Controls,
    wr: WindowRect,
    radius: f32,
    hue: f32,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let controls = Controls::new(vec![Control::slider(
        "radius",
        100.0,
        (10.0, 500.0),
        1.0,
    )]);

    let animation_script = AnimationScript::new(
        to_absolute_path(file!(), "./animation_script_test.toml"),
        Animation::new(MidiSongTiming::new(SKETCH_CONFIG.bpm)),
    );

    Model {
        controls,
        wr,
        radius: 0.0,
        hue: 0.0,
        animation_script,
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.animation_script.update();
    m.radius = m.animation_script.get("radius");
    // m.hue = m.animation_script.get("hue");
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .hsla(0.0, 0.0, 0.02, 0.1);

    draw.ellipse()
        .color(hsl(m.hue, 0.5, 0.5))
        .radius(m.radius)
        .x_y(0.0, 0.0);

    draw.to_frame(app, &frame).unwrap();
}
