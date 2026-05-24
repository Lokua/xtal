pub mod frame;
pub mod gpu;
pub mod graph;
pub mod mesh;
pub mod shader_watch;
pub mod uniforms;
#[cfg(feature = "video")]
pub mod video;
#[cfg(not(feature = "video"))]
pub mod video {
    use std::path::Path;

    pub struct VideoFrame {
        pub width: u32,
        pub height: u32,
        pub rgba: Vec<u8>,
    }

    pub struct VideoSource;

    impl VideoSource {
        pub fn new(_path: &Path) -> Result<Self, String> {
            Err("video resources require the xtal 'video' feature".to_string())
        }

        pub fn next_frame(&mut self) -> Result<Option<VideoFrame>, String> {
            Ok(None)
        }

        pub fn restart(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
}
