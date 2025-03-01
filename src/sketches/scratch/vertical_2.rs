use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "vertical_2",
    display_name: "Vertical 2",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(LegacySketchComponents)]
#[sketch(clear_color = "hsla(0.0, 0.2, 0.8, 0.5)")]
pub struct Model {
    controls: Controls,
    background_color: Rgba,
    noise: SimplexNoise,
}

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let controls = Controls::new(vec![Control::slider(
        "alpha",
        0.25,
        (0.001, 1.0),
        0.001,
    )]);

    Model {
        controls,
        background_color: hsla(0.0, 0.2, 0.8, 0.5).into(),
        noise: SimplexNoise::new(random::<u32>() * 10_000),
    }
}

pub fn update(_app: &App, _model: &mut Model, _update: Update) {}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let window_rect = app
        .window(frame.window_id())
        .expect("Unable to get window")
        .rect();

    let w = window_rect.w();
    let h = window_rect.h();
    let hh = h / 2.0;

    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(w, h)
        .color(model.background_color);

    let alpha = model.controls.float("alpha");
    let size = 1.0;
    let params = vec![
        (0.005, 10.0),
        (0.03, 12.0),
        (0.07, 14.0),
        (0.115, 16.0),
        (0.26, 18.0),
    ];

    for (ns, amp) in params.iter() {
        for y in (-hh as i32..=hh as i32).rev() {
            let y = y as f32;

            for i in 1..=*amp as i32 {
                let i = i as f32;
                let amp = y / (amp - i as f32);
                let x = model.noise.get([i * ns, y * ns]) * amp;
                let color = hsla(0.0, 0.0, 0.0, alpha);
                draw.rect().x_y(x, y).w_h(size, size).color(color);
            }
        }
    }

    draw.to_frame(app, &frame).unwrap();
}
