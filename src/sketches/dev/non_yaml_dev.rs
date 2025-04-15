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
};

const COUNT: usize = 512;

#[derive(SketchComponents)]
pub struct NonYamlDev {
    controls: ControlHub<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> NonYamlDev {
    let mut ui_controls: Vec<UiControl> = vec![];

    for i in 0..COUNT {
        ui_controls.push(UiControl::slider(
            &format!("{}", i),
            100.0,
            (10.0, 500.0),
            1.0,
        ));
    }

    let controls: ControlHub<Timing> = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .ui_controls(UiControls::new(&ui_controls))
        .build();

    NonYamlDev { controls }
}

impl Sketch for NonYamlDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        for i in 0..COUNT {
            self.controls.get(&format!("{}", i));
            // self.controls.animation.tri(i as f32);
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        // background
        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.4);

        draw.ellipse().color(ORANGERED).radius(100.0).x_y(0.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
