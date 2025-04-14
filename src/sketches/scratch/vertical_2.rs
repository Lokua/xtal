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

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(0.0, 0.2, 0.8, 0.5)")]
pub struct Vertical2 {
    controls: ControlHub<Timing>,
    background_color: Rgba,
    noise: SimplexNoise,
}

pub fn init(_app: &App, ctx: &Context) -> Vertical2 {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("alpha", 0.25, (0.001, 1.0), 0.001, None)
        .build();

    Vertical2 {
        controls,
        background_color: hsla(0.0, 0.2, 0.8, 0.5).into(),
        noise: SimplexNoise::new(random::<u32>() * 10_000),
    }
}

impl Sketch for Vertical2 {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .color(self.background_color);

        let alpha = self.controls.float("alpha");
        let size = 1.0;
        let params = [
            (0.005, 10.0),
            (0.03, 12.0),
            (0.07, 14.0),
            (0.115, 16.0),
            (0.26, 18.0),
        ];

        for (ns, amp) in params.iter() {
            for y in (-wr.hh() as i32..=wr.hh() as i32).rev() {
                let y = y as f32;

                for i in 1..=*amp as i32 {
                    let i = i as f32;
                    let amp = y / (amp - i);
                    let x = self.noise.get([i * ns, y * ns]) * amp;
                    let color = hsla(0.0, 0.0, 0.0, alpha);
                    draw.rect().x_y(x, y).w_h(size, size).color(color);
                }
            }
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
