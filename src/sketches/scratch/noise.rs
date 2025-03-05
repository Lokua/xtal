use nannou::color::*;
use nannou::noise::NoiseFn;
use nannou::noise::Seedable;
use nannou::noise::SuperSimplex;
use nannou::prelude::*;

use crate::framework::prelude::*;

// https://www.youtube.com/watch?v=0YvPgYDR1oM&list=PLeCiJGCSl7jc5UWvIeyQAvmCNc47IuwkM&index=6

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "noise",
    display_name: "Noise",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(0.0, 0.0, 1.0, 1.0)")]
pub struct Noise {
    animation: Animation<FrameTiming>,
    controls: Controls,
    noise: SuperSimplex,
    last_seed: u32,
}

pub fn init(_app: &App, _ctx: &LatticeContext) -> Noise {
    let animation =
        Animation::new(FrameTiming::new(Bpm::new(SKETCH_CONFIG.bpm)));

    let controls = Controls::new(vec![
        Control::checkbox("rotate", false),
        Control::slider("max_rect_length", 10.0, (1.0, 400.0), 1.0),
        Control::slider("rect_width", 1.5, (0.5, 10.0), 0.25),
        Control::slider("noise_scale", 3.0, (0.5, 10.0), 0.1),
        Control::slider("seed", 3.0, (3.0, 33_333.0), 33.0),
        Control::slider("angle_resolution", 45.0, (3.0, 180.0), 1.0),
        Control::slider("time_x", 1.0, (0.01, 10.0), 0.01),
    ]);

    let noise = SuperSimplex::new();
    let last_seed = noise.seed();

    Noise {
        animation,
        controls,
        noise,
        last_seed,
    }
}

impl Sketch for Noise {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        let seed = self.controls.float("seed") as u32;
        if seed != self.last_seed {
            self.noise = self.noise.set_seed(seed);
            self.last_seed = seed;
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let window_rect = ctx.window_rect();
        let draw = app.draw();

        if frame.nth() == 0 {
            draw.background().color(WHITE);
        }

        draw.rect()
            .w_h(window_rect.w(), window_rect.h())
            .color(hsla(0.0, 0.0, 1.0, 0.001));

        let circle_radius =
            self.animation.tri(8.0) * window_rect.w() * (2.0 / 3.0);
        let max_rect_length = self.controls.float("max_rect_length");
        let rect_width = self.controls.float("rect_width");
        let noise_scale = self.controls.float("noise_scale");
        let angle_resolution = self.controls.float("angle_resolution");
        let time_x = self.controls.float("time_x");
        let angle_increment = PI / angle_resolution;
        let total_segments = (360.0 / angle_increment) as i32;

        let draw_rotated = draw.rotate(if self.controls.bool("rotate") {
            self.animation.loop_phase(32.0) * PI * 2.0
        } else {
            0.0
        });

        for i in 0..total_segments {
            let current_angle = i as f32 * angle_increment;
            let noise_x = (current_angle.cos() + 1.0) * noise_scale;
            let noise_y = (current_angle.sin() + 1.0) * noise_scale;
            let noise_value = self.noise.get([
                noise_x as f64,
                noise_y as f64,
                (app.time * time_x) as f64,
            ]) as f32;
            let rect_length = (noise_value + 1.0) * (max_rect_length / 2.0);
            draw_rotated
                .rect()
                .color(hsl(0.3, 0.05, self.animation.tri(1.0)))
                .x_y(
                    circle_radius * current_angle.cos(),
                    circle_radius * current_angle.sin(),
                )
                .w_h(rect_length, rect_width)
                .rotate(current_angle);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
