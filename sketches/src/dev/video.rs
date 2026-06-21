use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH, WORKING_BPM};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "video",
    display_name: "Video",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 90.0,
    bpm: WORKING_BPM,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 24,
};

pub struct VideoSketch {
    video_shader_path: PathBuf,
    feedback_shader_path: PathBuf,
    video_paths: Vec<PathBuf>,
    control_script_path: PathBuf,
}

impl Sketch for VideoSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let current = graph.texture2d();
        let (ping, pong) = graph.feedback();
        let video = graph.video("a", self.video_paths.clone());

        graph
            .render()
            .shader(self.video_shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(video)
            .to(current);

        graph
            .render()
            .shader(self.feedback_shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(ping)
            .read(current)
            .to(pong);

        graph
            .render()
            .shader(self.feedback_shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(pong)
            .read(current)
            .to(ping);

        graph.present(ping);
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> VideoSketch {
    let assets = SketchAssets::from_file(file!());

    VideoSketch {
        video_shader_path: assets.wgsl(),
        feedback_shader_path: assets.path("video_feedback.wgsl"),
        video_paths: vec![
            PathBuf::from("clips/Rogers Park Trees (trimmed).mp4"),
            PathBuf::from("clips/Viaduct - Columbia Edge.mp4"),
            PathBuf::from("clips/Viaduct - Columbia Sky.mp4"),
            PathBuf::from("clips/Viaduct - Columbia.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Alley Drive.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Howard.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Lincoln Canal.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Main 2.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Main.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Noyes Park.mp4"),
            PathBuf::from("clips/Viaduct - Evanston Noyes.mp4"),
            PathBuf::from("clips/Viaduct - Farwell Faces Short.mp4"),
            PathBuf::from("clips/Viaduct - Farwell Faces.mp4"),
            PathBuf::from("clips/Viaduct - Farwell.mp4"),
            PathBuf::from("clips/Viaduct - Jarvis 2.mp4"),
            PathBuf::from("clips/Viaduct - Jarvis.mp4"),
            PathBuf::from("clips/Viaduct - Pratt & Glenwood.mp4"),
            PathBuf::from("clips/Viaduct - Pratt Fish.mp4"),
            PathBuf::from("clips/Viaduct - South Evanston.mp4"),
        ],
        control_script_path: assets.yaml(),
    }
}
