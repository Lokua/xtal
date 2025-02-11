use nannou::prelude::*;

use crate::framework::prelude::*;

// Live/2025/Lattice CV Test

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
    audio: AudioControls,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let audio = AudioControlBuilder::new()
        .with_buffer_processor(thru_buffer_processor)
        .control_from_config(
            "a",
            AudioControlConfig {
                channel: 0,
                slew_config: SlewConfig::default(),
                preemphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .control_from_config(
            "b",
            AudioControlConfig {
                channel: 1,
                slew_config: SlewConfig::default(),
                preemphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .build();

    Model { audio, wr }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    debug_throttled!(1_000, "a: {}, b: {}", m.audio.get("a"), m.audio.get("b"));
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .color(BLACK)
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h());

    let a = m.audio.get("a");
    let b = m.audio.get("b");

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
