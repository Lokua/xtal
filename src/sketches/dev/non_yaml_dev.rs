use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "non_yaml_dev",
    display_name: "ControlScript w/o YAML Dev",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(200),
};

#[derive(SketchComponents)]
pub struct NonYamlDev {
    controls: ControlScript<Timing>,
    radius: f32,
    hue: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> NonYamlDev {
    let controls: ControlScript<Timing> = ControlScriptBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("radius", 100.0, (10.0, 500.0), 1.0, None)
        .slider("unused", 100.0, (10.0, 500.0), 1.0, None)
        .slider_n("another", 100.0)
        .midi("x_pos", (0, 0), (-1.0, 1.0), 0.0)
        .midi_n("y_pos", (0, 1))
        .build();

    NonYamlDev {
        controls,
        radius: 0.0,
        hue: 0.0,
    }
}

impl Sketch for NonYamlDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.controls.update();
        let radius_max = self.controls.float("radius");

        self.radius = self.controls.animation.automate(
            &[
                Breakpoint::ramp(0.0, 10.0, Easing::Linear),
                Breakpoint::ramp(2.0, radius_max, Easing::Linear),
                Breakpoint::end(4.0, 10.0),
            ],
            Mode::Loop,
        );

        self.hue = self.controls.animation.tri(12.0)
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        // background
        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.4);

        draw.ellipse()
            .color(hsl(self.hue, 0.5, 0.5))
            .radius(self.radius)
            .x_y(
                map_range(
                    self.controls.get("x_pos"),
                    -1.0,
                    1.0,
                    -wr.hh(),
                    wr.hh(),
                ),
                0.0,
            );

        draw.to_frame(app, &frame).unwrap();
    }
}
