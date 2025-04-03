use nannou::color::named::*;
use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "audio_dev",
    display_name: "Audio Test",
    fps: 30.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
    play_mode: PlayMode::Loop,
};

const N_BANDS: usize = 8;

#[derive(SketchComponents)]
pub struct AudioDev {
    controls: ControlHub<Timing>,
    audio: Audio,
    fft_bands: Vec<f32>,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> AudioDev {
    let audio = Audio::new();

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("pre_emphasis", 0.88, (0.0, 1.0), 0.001, None)
        .slider("rise", 0.96, (0.001, 1.0), 0.001, None)
        .slider("fall", 0.9, (0.0, 1.0), 0.001, None)
        .build();

    AudioDev {
        controls,
        audio,
        fft_bands: Vec::new(),
    }
}

impl Sketch for AudioDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.controls.update();
        self.fft_bands = self.audio.bands(
            N_BANDS,
            30.0,
            10_000.0,
            self.controls.get("pre_emphasis"),
            self.controls.get("rise"),
            self.controls.get("fall"),
        );
        // debug_throttled!(1_000, "fft_bands: {:?}", self.fft_bands);
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        frame.clear(BLACK);
        draw.background().color(rgb(0.2, 0.2, 0.2));

        let gradient: Gradient<LinSrgb> = Gradient::new(vec![
            PURPLE.into_lin_srgb(),
            GREEN.into_lin_srgb(),
            LIGHTBLUE.into_lin_srgb(),
        ]);

        let start_x = -wr.w() / 2.0;
        let cell_pad = 0.0;
        let cell_size = wr.w() / self.fft_bands.len() as f32;
        for (index, band) in self.fft_bands.iter().enumerate() {
            draw.rect()
                .x_y(
                    start_x + index as f32 * cell_size + cell_size / 2.0,
                    -wr.h() / 2.0 + (band * wr.h()) / 2.0,
                )
                .w_h(cell_size - cell_pad, band * wr.h())
                .color(
                    gradient.get(index as f32 / self.fft_bands.len() as f32),
                );
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
