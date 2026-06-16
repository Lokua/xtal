//! Rendering primitives used by the xtal runtime.
//!
//! Sketches usually interact with this module through `GraphBuilder`, `Mesh`,
//! and `UniformBanks`. The runtime compiles the resulting graph into GPU
//! pipelines with `gpu`, then executes that compiled graph into a `Frame`.
//!
//! Module roles:
//!
//! - `graph` is the sketch-facing render/compute graph recipe API;
//! - `mesh` describes vertex buffers for render passes;
//! - `uniforms` owns the shader parameter bank buffer;
//! - `frame` wraps one acquired surface texture and command encoder;
//! - `shader_watch` reports WGSL file changes for hot reload;
//! - `video` decodes video frames for graph texture resources;
//! - `gpu` compiles and executes graph specs.

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
    //! No-op video backend used when the `video` feature is disabled.
    //!
    //! The graph API remains available so callers can compile without feature
    //! gating every video reference. Opening a video source returns an error;
    //! the rest of the methods are inert to preserve the compiled interface.

    use std::path::PathBuf;

    use crate::control::VideoTransport;

    /// RGBA frame payload shape shared with the real video backend.
    pub struct VideoFrame {
        pub width: u32,
        pub height: u32,
        pub rgba: Vec<u8>,
    }

    /// Placeholder video source for builds without GStreamer support.
    pub struct VideoSource;

    impl VideoSource {
        /// Returns an error because video decoding is not compiled in.
        pub fn new(_paths: &[PathBuf]) -> Result<Self, String> {
            Err("video resources require the xtal 'video' feature".to_string())
        }

        /// Accepts transport updates without doing work.
        pub fn apply_transport(
            &mut self,
            _transport: &VideoTransport,
            _beats: f32,
            _bpm: f32,
        ) -> Result<(), String> {
            Ok(())
        }

        /// Produces no frames in no-video builds.
        pub fn next_frame(&mut self) -> Result<Option<VideoFrame>, String> {
            Ok(None)
        }

        /// Keeps the graph API available when the optional backend is off.
        pub fn restart(&mut self) -> Result<(), String> {
            Ok(())
        }

        /// Keeps the graph API available when the optional backend is off.
        pub fn restart_with_transport(
            &mut self,
            _transport: &VideoTransport,
            _bpm: f32,
        ) -> Result<(), String> {
            Ok(())
        }
    }
}
