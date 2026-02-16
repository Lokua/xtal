use std::path::PathBuf;

use crate::context::Context;
use crate::frame::Frame;
use crate::graph::GraphBuilder;

pub struct SketchConfig {
    pub name: &'static str,
    pub display_name: &'static str,
    pub fps: f32,
    pub w: u32,
    pub h: u32,
    pub banks: usize,
}

pub trait Sketch {
    fn setup(&self, graph: &mut GraphBuilder);

    fn update(&mut self, _ctx: &Context) {}

    fn view(&mut self, _frame: &mut Frame, _ctx: &Context) {}
}

pub struct FullscreenShaderSketch {
    shader_path: PathBuf,
}

impl FullscreenShaderSketch {
    pub fn new(shader_path: impl Into<PathBuf>) -> Self {
        Self {
            shader_path: shader_path.into(),
        }
    }
}

impl Sketch for FullscreenShaderSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");

        graph
            .render("main")
            .shader(self.shader_path.clone())
            .read("params")
            .write("surface")
            .add();

        graph.present("surface");
    }
}
