#[path = "runtime/context.rs"]
pub mod context;
pub mod control;
#[path = "render/frame.rs"]
pub mod frame;
pub mod framework;
#[path = "render/gpu.rs"]
pub mod gpu;
#[path = "render/graph.rs"]
pub mod graph;
pub mod motion;
pub mod prelude;
#[path = "sketches/registration_macros.rs"]
mod registration_macros;
pub mod runtime;
#[path = "render/shader_watch.rs"]
pub mod shader_watch;
#[path = "sketches/sketch.rs"]
pub mod sketch;
#[path = "sketches/sketch_assets.rs"]
pub mod sketch_assets;
#[path = "render/uniforms.rs"]
pub mod uniforms;

pub use runtime::app::{
    run_registry, run_registry_with_channels, run_registry_with_web_view,
};
