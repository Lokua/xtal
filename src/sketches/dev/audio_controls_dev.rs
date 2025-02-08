use nannou::prelude::*;

use crate::framework::{
    audio_controls::{AudioControlBuilder, AudioControls},
    prelude::*,
};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "audio_controls_dev",
    display_name: "Audio Controls Test",
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
    controls: Controls,
    audio: AudioControls,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let controls = Controls::new(vec![
        Control::slider_norm("rise", 0.0),
        Control::slider_norm("fall", 0.0),
    ]);

    let audio = AudioControlBuilder::new()
        .control_mapped("bd", 0, (0.5, 0.5), 0.0, (0.0, 400.0), 0.0)
        .control_mapped("hh", 1, (0.5, 0.5), 0.0, (0.0, 400.0), 0.0)
        .build();

    Model {
        audio,
        controls,
        wr,
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    debug_throttled!(500, "a: {}, b: {}", m.audio.get("bd"), m.audio.get("hh"));

    if m.controls.changed() {
        m.audio.update_all_slew(SlewConfig::new(
            m.controls.float("rise"),
            m.controls.float("fall"),
        ));

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .color(BLACK)
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h());

    let a = m.audio.get("bd");
    let b = m.audio.get("hh");

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
