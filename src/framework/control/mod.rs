//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.
//!
#![doc = include_str!("../../../docs/control_script_reference.md")]

pub mod control_hub;

pub mod audio_controls;
mod config;
pub mod control_hub_builder;
pub mod control_provider;
mod dep_graph;
mod eval_cache;
pub mod midi_controls;
pub mod osc_controls;
mod param_mod;
pub mod serialization;
pub mod ui_controls;

pub use audio_controls::*;
pub use control_hub::*;
pub use control_hub_builder::ControlHubBuilder;
pub use control_provider::*;
pub use midi_controls::*;
pub use osc_controls::*;
pub use serialization::*;
pub use ui_controls::*;
