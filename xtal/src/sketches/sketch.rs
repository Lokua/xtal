use std::path::PathBuf;

use crate::context::Context;
use crate::frame::Frame;
use crate::graph::GraphBuilder;

pub struct SketchConfig {
    pub name: &'static str,
    pub display_name: &'static str,
    pub play_mode: PlayMode,
    pub fps: f32,
    pub bpm: f32,
    pub w: u32,
    pub h: u32,
    pub banks: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayMode {
    Loop,
    Pause,
    Advance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingMode {
    Frame,
    Osc,
    Midi,
    Hybrid,
    Manual,
}

pub trait Sketch {
    fn setup(&self, graph: &mut GraphBuilder);

    fn control_script(&self) -> Option<PathBuf> {
        None
    }

    fn timing_mode(&self) -> TimingMode {
        TimingMode::Frame
    }

    fn update(&mut self, _ctx: &Context) {}

    fn view(&mut self, _frame: &mut Frame, _ctx: &Context) {}
}

pub struct FullscreenShaderSketch {
    shader_path: PathBuf,
    control_script_path: Option<PathBuf>,
    timing_mode: TimingMode,
}

impl FullscreenShaderSketch {
    pub fn new(shader_path: impl Into<PathBuf>) -> Self {
        Self {
            shader_path: shader_path.into(),
            control_script_path: None,
            timing_mode: TimingMode::Frame,
        }
    }

    pub fn with_control_script(
        mut self,
        control_script_path: impl Into<PathBuf>,
    ) -> Self {
        self.control_script_path = Some(control_script_path.into());
        self
    }

    pub fn with_timing_mode(mut self, timing_mode: TimingMode) -> Self {
        self.timing_mode = timing_mode;
        self
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

    fn control_script(&self) -> Option<PathBuf> {
        self.control_script_path.clone()
    }

    fn timing_mode(&self) -> TimingMode {
        self.timing_mode
    }
}
