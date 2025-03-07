use nannou::prelude::*;

use crate::framework::prelude::*;

// ~/Documents/Live/2024/Lattice MIDI Test

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
pub struct MidiDev {
    midi: MidiControls,
}

pub fn init(_app: &App, _ctx: &LatticeContext) -> MidiDev {
    let midi = MidiControlBuilder::new()
        .control_n("a", (0, 1), 0.5)
        .control_n("b", (0, 2), 0.5)
        .build();

    MidiDev { midi }
}

impl Sketch for MidiDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        // debug!("{}", self.midi.get("a"));
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.1);

        let b = self.midi.get("b");
        draw.ellipse()
            .no_fill()
            .stroke(hsl(map_range(b, 0.0, 1.0, 0.7, 1.0), 0.5, 1.0 - (b * 0.5)))
            .stroke_weight(5.0)
            .radius(map_range(sigmoid(b, 12.0), 0.0, 1.0, 1.0, 400.0))
            .x_y(0.0, 0.0);

        let a = self.midi.get("a");
        draw.ellipse()
            .no_fill()
            .stroke(hsl(0.5, 0.3, a * 0.5))
            .stroke_weight(10.0)
            .radius(map_range(sigmoid(a, 12.0), 0.0, 1.0, 1.0, 200.0))
            .x_y(0.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
