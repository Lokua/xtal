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
    controls: Controls,
    audio: Audio,
    fft_bands: Vec<f32>,
}

pub fn init(_app: &App, _ctx: LatticeContext) -> AudioDev {
    let audio =
        Audio::new(crate::config::AUDIO_DEVICE_SAMPLE_RATE, SKETCH_CONFIG.fps);

    let controls = Controls::new(vec![
        Control::Slider {
            name: "pre_emphasis".to_string(),
            value: 0.88,
            min: 0.0,
            max: 1.0,
            step: 0.001,
            disabled: None,
        },
        Control::Slider {
            name: "rise".to_string(),
            value: 0.96,
            min: 0.001,
            max: 1.0,
            step: 0.001,
            disabled: None,
        },
        Control::Slider {
            name: "fall".to_string(),
            value: 0.9,
            min: 0.0,
            max: 1.0,
            step: 0.001,
            disabled: None,
        },
    ]);

    AudioDev {
        controls,
        audio,
        fft_bands: Vec::new(),
    }
}

impl Sketch for AudioDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.fft_bands = self.audio.bands(
            N_BANDS,
            30.0,
            10_000.0,
            self.controls.float("pre_emphasis"),
            self.controls.float("rise"),
            self.controls.float("fall"),
        );
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
                .w_h(cell_size as f32 - cell_pad, band * wr.h())
                .color(
                    gradient.get(index as f32 / self.fft_bands.len() as f32),
                );
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
