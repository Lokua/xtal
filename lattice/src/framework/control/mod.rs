//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.
pub mod control_hub;

pub mod audio_controls;
mod config;
pub mod control_hub_builder;
pub mod control_hub_provider;
pub mod control_traits;
mod dep_graph;
mod eval_cache;
pub mod midi_controls;
pub mod osc_controls;
mod param_mod;
pub mod ui_controls;

pub use audio_controls::*;
pub use control_hub::*;
#[allow(unused_imports)]
pub use control_hub_builder::*;
pub use control_hub_provider::*;
pub use control_traits::*;
pub use midi_controls::*;
pub use osc_controls::*;
pub use ui_controls::*;
