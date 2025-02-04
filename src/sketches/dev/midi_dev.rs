use nannou::prelude::*;

use crate::framework::prelude::*;

// b/w Live -> 2014.12.12 Lattice MIDI Test

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "midi_dev",
    display_name: "MIDI Test",
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
    midi: MidiControls,
}

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let midi = MidiControlBuilder::new()
        .control("a", (0, 1), 0.5)
        .control("b", (0, 2), 0.5)
        .build();

    Model { midi }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    debug!("{}", m.midi.get("a"));
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let window_rect = app
        .window(frame.window_id())
        .expect("Unable to get window")
        .rect();

    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(window_rect.w(), window_rect.h())
        .hsla(0.0, 0.0, 0.02, 0.1);

    let b = model.midi.get("b");
    draw.ellipse()
        .no_fill()
        .stroke(hsl(map_range(b, 0.0, 1.0, 0.7, 1.0), 0.5, 1.0 - (b * 0.5)))
        .stroke_weight(5.0)
        .radius(map_range(sigmoid(b, 12.0), 0.0, 1.0, 1.0, 400.0))
        .x_y(0.0, 0.0);

    let a = model.midi.get("a");
    draw.ellipse()
        .no_fill()
        .stroke(hsl(0.5, 0.3, a * 0.5))
        .stroke_weight(10.0)
        .radius(map_range(sigmoid(a, 12.0), 0.0, 1.0, 1.0, 200.0))
        .x_y(0.0, 0.0);

    draw.to_frame(app, &frame).unwrap();
}
