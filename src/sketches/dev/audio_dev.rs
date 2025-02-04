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

const SAMPLE_RATE: usize = 48_000;
const N_BANDS: usize = 8;

#[derive(SketchComponents)]
pub struct Model {
    controls: Controls,
    audio: Audio,
    fft_bands: Vec<f32>,
}

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let audio = Audio::new(SAMPLE_RATE, SKETCH_CONFIG.fps);

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
            name: "rise_rate".to_string(),
            value: 0.96,
            min: 0.001,
            max: 1.0,
            step: 0.001,
            disabled: None,
        },
        Control::Slider {
            name: "fall_rate".to_string(),
            value: 0.9,
            min: 0.0,
            max: 1.0,
            step: 0.001,
            disabled: None,
        },
    ]);

    Model {
        controls,
        audio,
        fft_bands: Vec::new(),
    }
}

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    model.fft_bands = model.audio.bands(
        N_BANDS,
        30.0,
        10_000.0,
        model.controls.float("pre_emphasis"),
        model.controls.float("rise_rate"),
        model.controls.float("fall_rate"),
    );
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let w = SKETCH_CONFIG.w as f32;
    let h = SKETCH_CONFIG.h as f32;
    let draw = app.draw();

    frame.clear(BLACK);
    draw.background().color(rgb(0.2, 0.2, 0.2));

    let gradient: Gradient<LinSrgb> = Gradient::new(vec![
        PURPLE.into_lin_srgb(),
        GREEN.into_lin_srgb(),
        LIGHTBLUE.into_lin_srgb(),
    ]);

    let start_x = -w / 2.0;
    let cell_pad = 0.0;
    let cell_size = w / model.fft_bands.len() as f32;
    for (index, band) in model.fft_bands.iter().enumerate() {
        draw.rect()
            .x_y(
                start_x + index as f32 * cell_size + cell_size / 2.0,
                -h / 2.0 + (band * h) / 2.0,
            )
            .w_h(cell_size as f32 - cell_pad, band * h)
            .color(gradient.get(index as f32 / model.fft_bands.len() as f32));
    }

    draw.to_frame(app, &frame).unwrap();
}
