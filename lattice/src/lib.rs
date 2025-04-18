//! A framework build around [Nannou][nannou] with a slick UI. See the project's
//! [repo][repo] for an overview.
//!
//! [nannou]: https://github.com/nannou-org/nannou
//! [repo]: https://github.com/lokua/lattice

pub use lattice_derives::*;

pub(crate) mod framework;
pub(crate) mod runtime;
pub(crate) use runtime::global;

/// Run the app after registering your sketches with [`register`]
pub use crate::runtime::app::run;

#[doc(hidden)]
pub use crate::runtime::registry::REGISTRY;

#[doc(hidden)]
pub mod internal {
    pub use crate::framework::midi::{self};
    pub use crate::runtime::web_view_process::run as run_web_view;
}

/// The recommended single import for all critical functionality
pub mod prelude {
    pub use crate::framework::audio::Audio;
    pub use crate::framework::control::audio_controls::*;
    pub use crate::framework::control::control_hub::*;
    pub use crate::framework::control::control_hub_builder::*;
    pub use crate::framework::control::control_hub_provider::*;
    pub use crate::framework::control::control_traits::*;
    pub use crate::framework::control::midi_controls::*;
    pub use crate::framework::control::osc_controls::*;
    pub use crate::framework::control::ui_controls::*;
    pub use crate::framework::gpu;
    pub use crate::framework::motion::*;
    pub use crate::framework::noise::*;
    pub use crate::framework::sketch::*;
    pub use crate::framework::util::*;
    pub use crate::framework::window_rect::WindowRect;
    pub use crate::register;
    pub use crate::runtime::app::run;
    pub use crate::ternary;
    pub use lattice_derives::{SketchComponents, uniforms};

    #[cfg(feature = "logging")]
    pub use crate::debug_once;
    #[cfg(feature = "logging")]
    pub use crate::debug_throttled;
    #[cfg(feature = "logging")]
    pub use crate::framework::logging::*;
    #[cfg(feature = "logging")]
    pub use crate::warn_once;
}

/// Control sketch parameters with UI controls, MIDI, OSC, and audio
pub mod control {
    pub use crate::framework::control::audio_controls::*;
    pub use crate::framework::control::control_hub::*;
    pub use crate::framework::control::control_hub_builder::*;
    pub use crate::framework::control::control_traits::*;
    pub use crate::framework::control::midi_controls::*;
    pub use crate::framework::control::osc_controls::*;
    pub use crate::framework::control::ui_controls::*;
}

/// Timing, animation, and easing methods
pub mod motion {
    pub use crate::framework::motion::*;
}

/// A dumping ground for miscellaneous helpers
pub mod util {
    pub use crate::framework::util::to_absolute_path;
}
