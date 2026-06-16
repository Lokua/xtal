//! Runtime control primitives, collections, and YAML schema support.
//!
//! The control module is the parameter layer behind `ControlHub`. It contains
//! concrete control collections for UI, MIDI, OSC, and audio input; YAML
//! deserialization types; hot-parameter dependency tracking; MIDI learn state;
//! and small transport structs used by video controls.

/// Audio input controls backed by `cpal`.
pub mod audio_controls;
/// YAML control script schema and deserialization helpers.
mod config;
/// Runtime control hub that combines all control sources.
pub mod control_hub;
/// Shared traits implemented by concrete control collections.
pub mod control_traits;
/// Hot-parameter dependency graph.
mod dep_graph;
/// Per-frame cache for evaluated hot parameters.
mod eval_cache;
/// MIDI-learn mapping mode state.
pub mod map_mode;
/// MIDI CC controls and outbound CC message encoding.
pub mod midi_controls;
/// OSC controls backed by the shared OSC receiver.
pub mod osc_controls;
/// Hot-parameter resolution helpers for animations and effects.
mod param_mod;
/// UI-facing control definitions and values.
pub mod ui_controls;
/// Video control transport state.
pub mod video_transport;

/// Re-export audio control types for `crate::control::*` callers.
pub use audio_controls::*;
/// Re-export the main `ControlHub` API.
pub use control_hub::*;
/// Re-export shared control collection traits.
pub use control_traits::*;
/// Re-export MIDI control types.
pub use midi_controls::*;
/// Re-export OSC control types.
pub use osc_controls::*;
/// Re-export UI control types.
pub use ui_controls::*;
/// Re-export video transport types.
pub use video_transport::*;
