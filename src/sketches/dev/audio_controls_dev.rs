use nannou::prelude::*;

use crate::framework::prelude::*;

// ~/Documents/Live/2025/Lattice Audio Controls Test Project/Lattice Audio Controls Test.als

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "audio_controls_dev",
    display_name: "Audio Controls Test",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct AudioControlsDev {
    controls: Controls,
    audio: AudioControls,
}

pub fn init(_app: &App, _ctx: LatticeContext) -> AudioControlsDev {
    let controls = Controls::with_previous(vec![
        Control::slide("pre_emphasis", 0.0),
        Control::slide("detect", 0.0),
        Control::slide("rise", 0.0),
        Control::slide("fall", 0.0),
    ]);

    let audio = AudioControlBuilder::new()
        .control_from_config(
            "bd",
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
            "hh",
            AudioControlConfig {
                channel: 1,
                slew_limiter: SlewLimiter::default(),
                pre_emphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .control_from_config(
            "chord",
            AudioControlConfig {
                channel: 2,
                slew_limiter: SlewLimiter::default(),
                pre_emphasis: 0.0,
                detect: 0.0,
                range: (0.0, 700.0),
                default: 0.0,
            },
        )
        .build();

    AudioControlsDev { audio, controls }
}

impl Sketch for AudioControlsDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        // debug_throttled!(500, "a: {}, b: {}", self.audio.get("bd"), self.audio.get("hh"));

        if self.controls.changed() {
            let pre_emphasis = self.controls.float("pre_emphasis");
            let detect = self.controls.float("detect");
            let rise = self.controls.float("rise");
            let fall = self.controls.float("fall");

            self.audio.update_controls(|control| {
                control.pre_emphasis = pre_emphasis;
                control.detect = detect;
                control.slew_limiter.set_rates(rise, fall);
            });

            self.controls.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().color(WHITE).x_y(0.0, 0.0).w_h(wr.w(), wr.h());

        let bd = self.audio.get("bd");
        let hh = self.audio.get("hh");
        let chord = self.audio.get("chord");

        draw.ellipse()
            .color(rgba(0.02, 0.02, 0.02, 0.9))
            .radius(bd)
            .x_y(-wr.w() / 4.0, 0.0);

        draw.ellipse()
            .color(rgba(0.8, 0.2, 0.8, 0.9))
            .radius(chord)
            .x_y(0.0, 0.0);

        draw.ellipse()
            .color(rgba(0.5, 0.5, 0.5, 0.9))
            .radius(hh)
            .x_y(wr.w() / 4.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
