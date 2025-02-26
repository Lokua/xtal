use std::collections::HashMap;

use nannou::prelude::*;

use crate::framework::prelude::*;
use crate::sketches::animation_dev;
use crate::sketches::control_script_dev;

// WIP. This sketch isn't really working correctly. I believe it's because the
// connection to controls is being broken (the main app passes controls to
// _this_ model so the underlying sketch's controls are never updated)

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "multiple_sketches_dev",
    display_name: "Multiple Sketches POC",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

#[derive(SketchComponents)]
pub struct Model {
    wr: WindowRect,
    controls: Controls,
    initialized: HashMap<String, bool>,
    animation_dev_model: Option<animation_dev::Model>,
    control_script_dev_model: Option<control_script_dev::Model>,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = Controls::new(vec![Control::select(
        "sketch",
        "animation_dev",
        &["animation_dev", "control_script_dev"],
    )]);

    let mut initialized = HashMap::new();
    initialized.insert("animation_dev".to_string(), false);
    initialized.insert("control_script_dev".to_string(), false);

    let mut animation_dev_model = None;
    let mut control_script_dev_model = None;

    match controls.string("sketch").as_str() {
        "animation_dev" => {
            let model = animation_dev::init_model(app, wr.clone());
            animation_dev_model = Some(model);
            initialized.insert("animation_dev".to_string(), true);
        }
        "control_script_dev" => {
            let model = control_script_dev::init_model(app, wr.clone());
            control_script_dev_model = Some(model);
            initialized.insert("control_script_dev".to_string(), true);
        }
        _ => panic!(),
    }

    Model {
        wr,
        initialized,
        controls,
        animation_dev_model,
        control_script_dev_model,
    }
}

pub fn update(app: &App, m: &mut Model, update: Update) {
    match m.controls.string("sketch").as_str() {
        "animation_dev" => {
            if !m.initialized.get("animation_dev").unwrap_or(&false) {
                let mut model = animation_dev::init_model(app, m.wr.clone());
                m.controls.retain(|control| control.name() == "sketch");
                if let Some(model_controls) = model.controls() {
                    for control in model_controls.get_controls_mut().drain(..) {
                        m.controls.add(control);
                    }
                }
                m.animation_dev_model = Some(model);
                m.initialized.insert("animation_dev".to_string(), true);
            }
            if let Some(model) = &mut m.animation_dev_model {
                animation_dev::update(app, model, update);
            }
        }
        "control_script_dev" => {
            if !m.initialized.get("control_script_dev").unwrap_or(&false) {
                let mut model =
                    control_script_dev::init_model(app, m.wr.clone());
                m.controls.retain(|control| control.name() == "sketch");
                if let Some(model_controls) = model.controls() {
                    for control in model_controls.get_controls_mut().drain(..) {
                        m.controls.add(control);
                    }
                }
                m.control_script_dev_model = Some(model);
                m.initialized.insert("control_script_dev".to_string(), true);
            }
            if let Some(model) = &mut m.control_script_dev_model {
                control_script_dev::update(app, model, update);
            }
        }
        _ => panic!(),
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(BLACK);

    match m.controls.string("sketch").as_str() {
        "animation_dev" => {
            if let Some(model) = &m.animation_dev_model {
                animation_dev::view(app, model, frame);
            }
        }
        "control_script_dev" => {
            if let Some(model) = &m.control_script_dev_model {
                control_script_dev::view(app, model, frame);
            }
        }
        _ => panic!(),
    }
}
