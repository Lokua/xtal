//! Video playback transport values derived from control scripts.

/// Playback direction for video controls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum VideoDirection {
    /// Advance frames forward.
    #[default]
    Forward,
    /// Advance frames backward.
    Backward,
    /// Alternate forward and backward over each cycle.
    PingPong,
}

/// Resolved transport state for one video source.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoTransport {
    /// Video source identifier or path from the control script.
    pub source: String,
    /// Selected clip or frame index.
    pub index: usize,
    /// Start position in beats.
    pub start: f32,
    /// Playback duration in beats.
    pub beats: f32,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Playback direction mode.
    pub direction: VideoDirection,
}
