use nannou::prelude::*;

use crate::framework::prelude::*;

// Live/2025/Lattice CV Test

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "cv_test",
    display_name: "CV Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct CvTest {
    audio: AudioControls,
}

pub fn init(_app: &App, _ctx: &Context) -> CvTest {
    let audio = AudioControlBuilder::new()
        .with_buffer_processor(thru_buffer_processor)
        .control_from_config(
            "a",
            AudioControlConfig {
                channel: 0,
                slew_limiter: SlewLimiter::default(),
                pre_emphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .control_from_config(
            "b",
            AudioControlConfig {
                channel: 1,
                slew_limiter: SlewLimiter::default(),
                pre_emphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .build();

    CvTest { audio }
}

impl Sketch for CvTest {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        debug_throttled!(
            1_000,
            "a: {}, b: {}",
            self.audio.get("a"),
            self.audio.get("b")
        );
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().color(BLACK).x_y(0.0, 0.0).w_h(wr.w(), wr.h());

        let a = self.audio.get("a");
        let b = self.audio.get("b");

        draw.ellipse()
            .color(rgba(1.0, 0.0, 0.0, 0.5))
            .radius(a)
            .x_y(-wr.w() / 16.0, 0.0);

        draw.ellipse()
            .color(rgba(0.0, 0.0, 1.0, 0.5))
            .radius(b)
            .x_y(wr.w() / 16.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
