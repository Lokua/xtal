pub mod app;
pub mod events;
pub mod frame_recorder;
pub mod recording;
pub mod registry;
pub mod serialization;
pub mod storage;
pub mod tap_tempo;
pub mod web_view;
pub mod web_view_bridge;

/// True when the transitional legacy runtime path is compiled in.
pub const LEGACY_RUNTIME_ENABLED: bool = cfg!(feature = "legacy_runtime");

/// True when the xtal2 runtime path is compiled in.
pub const XTAL2_RUNTIME_ENABLED: bool = cfg!(feature = "xtal2");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeFlavor {
    Legacy,
    Xtal2,
}

pub fn available_flavors() -> Vec<RuntimeFlavor> {
    let mut out = Vec::new();

    if LEGACY_RUNTIME_ENABLED {
        out.push(RuntimeFlavor::Legacy);
    }

    if XTAL2_RUNTIME_ENABLED {
        out.push(RuntimeFlavor::Xtal2);
    }

    out
}
