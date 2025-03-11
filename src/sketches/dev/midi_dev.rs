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
        .control_n("a", (0, 0), 0.5)
        .control_n("b", (0, 1), 0.5)
        .control_n("c", (0, 2), 0.5)
        .control_n("d", (0, 127), 0.5)
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

        let width = 40.0;
        let pad = 20.0;

        draw.rect().color(CYAN).w_h(width, 10.0).x_y(
            //
            -wr.hw() + pad,
            -wr.hh() + self.midi.get("a") * wr.h(),
        );

        draw.rect().color(TURQUOISE).w_h(width, 10.0).x_y(
            -wr.hh() + pad + wr.qw(),
            -wr.hh() + self.midi.get("b") * wr.h(),
        );

        draw.rect().color(LIGHTSTEELBLUE).w_h(width, 10.0).x_y(
            //
            wr.qw() - pad,
            -wr.hh() + self.midi.get("c") * wr.h(),
        );

        draw.rect().color(CORNFLOWERBLUE).w_h(width, 10.0).x_y(
            //
            wr.hh() - pad,
            -wr.hh() + self.midi.get("d") * wr.h(),
        );

        draw.to_frame(app, &frame).unwrap();
    }
}
