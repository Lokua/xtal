#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum VideoDirection {
    #[default]
    Forward,
    Backward,
    PingPong,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoTransport {
    pub source: String,
    pub start: f32,
    pub beats: f32,
    pub speed: f32,
    pub direction: VideoDirection,
}
