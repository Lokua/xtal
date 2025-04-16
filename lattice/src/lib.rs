pub use lattice_derives::*;

pub(crate) mod config;
pub(crate) mod framework;
pub(crate) mod global;
pub mod runtime;

pub mod internal {
    pub use crate::runtime::web_view_process::run as run_web_view;
}

pub mod prelude {
    pub use crate::debug_throttled;
    pub use crate::framework::audio::Audio;
    pub use crate::framework::control::*;
    pub use crate::framework::distance::{self};
    pub use crate::framework::gpu;
    pub use crate::framework::logging::*;
    pub use crate::framework::motion::*;
    pub use crate::framework::noise::*;
    pub use crate::framework::sketch::*;
    // TODO: a lot of this should be moved into the sketches package
    pub use crate::framework::util::{
        CUBE_POSITIONS, IntoLinSrgb, PHI_F32, QUAD_POSITIONS, TWO_PI,
        average_neighbors, bool_to_f32, chaikin, circle_contains_point,
        create_grid, lerp, lin_srgb_to_lin_srgba, luminance, map_clamp,
        multi_lerp, nearby_point, random_normal, rect_contains_point,
        safe_range, to_absolute_path, triangle_map, trig_fn_lookup,
    };
    pub use crate::framework::window_rect::WindowRect;
    pub use crate::register;
    pub use crate::runtime::app::run;
    pub use crate::str_vec;
    pub use crate::ternary;
    pub use lattice_derives::{SketchComponents, uniforms};
}

// pub mod core {
//     pub use crate::framework::audio::*;
//     pub use crate::framework::distance::*;
//     pub use crate::framework::gpu::*;
//     pub use crate::framework::noise::*;
//     pub use crate::framework::sketch::*;
//     pub use crate::framework::util::*;
//     pub use crate::framework::window_rect::*;
//     pub use crate::runtime::app::run;
// }

// pub mod control {
//     pub use crate::framework::control::audio_controls::*;
//     pub use crate::framework::control::control_hub::*;
//     pub use crate::framework::control::control_hub_builder::*;
//     pub use crate::framework::control::midi_controls::*;
//     pub use crate::framework::control::osc_controls::*;
//     pub use crate::framework::control::ui_controls::*;
// }

// pub mod logging {
//     pub use crate::framework::logging::*;
// }

pub mod midi {
    pub use crate::framework::midi::{
        list_input_ports, list_output_ports, print_ports,
    };
}

// pub mod timing {
//     pub use crate::framework::motion::animation::*;
//     pub use crate::framework::motion::easing::*;
//     pub use crate::framework::motion::effects::*;
//     pub use crate::framework::motion::timing::*;
// }
