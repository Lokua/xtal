use nannou::prelude::*;

use crate::framework::prelude::*;

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

#[derive(LegacySketchComponents)]
pub struct Model {
    controls: Controls,
    audio: AudioControls,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
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

    Model {
        audio,
        controls,
        wr,
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    // debug_throttled!(500, "a: {}, b: {}", m.audio.get("bd"), m.audio.get("hh"));

    if m.controls.changed() {
        let pre_emphasis = m.controls.float("pre_emphasis");
        let detect = m.controls.float("detect");
        let rise = m.controls.float("rise");
        let fall = m.controls.float("fall");

        m.audio.update_controls(|control| {
            control.pre_emphasis = pre_emphasis;
            control.detect = detect;
            control.slew_limiter.set_rates(rise, fall);
        });

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .color(WHITE)
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h());

    let bd = m.audio.get("bd");
    let hh = m.audio.get("hh");
    let chord = m.audio.get("chord");

    draw.ellipse()
        .color(rgba(0.02, 0.02, 0.02, 0.9))
        .radius(bd)
        .x_y(-m.wr.w() / 4.0, 0.0);

    draw.ellipse()
        .color(rgba(0.8, 0.2, 0.8, 0.9))
        .radius(chord)
        .x_y(0.0, 0.0);

    draw.ellipse()
        .color(rgba(0.5, 0.5, 0.5, 0.9))
        .radius(hh)
        .x_y(m.wr.w() / 4.0, 0.0);

    draw.to_frame(app, &frame).unwrap();
}
