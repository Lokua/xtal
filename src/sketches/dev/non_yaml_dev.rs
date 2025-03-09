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

const COUNT: usize = 512;

#[derive(SketchComponents)]
pub struct NonYamlDev {
    controls: ControlScript<Timing>,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> NonYamlDev {
    let mut ui_controls: Vec<Control> = vec![];

    for i in 0..COUNT {
        ui_controls.push(Control::slider(
            &format!("{}", i),
            100.0,
            (10.0, 500.0),
            1.0,
        ));
    }

    let controls: ControlScript<Timing> = ControlScriptBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .ui_controls(UiControls::new(ui_controls))
        .build();

    NonYamlDev { controls }
}

impl Sketch for NonYamlDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.controls.update();
        for i in 0..COUNT {
            self.controls.get(&format!("{}", i));
            // self.controls.animation.tri(i as f32);
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
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
