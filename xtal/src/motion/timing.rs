use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::time::frame_clock;
use crate::core::util::AtomicF32;

#[derive(Clone, Debug)]
pub struct Bpm(Arc<AtomicF32>);

impl Bpm {
    pub fn new(bpm: f32) -> Self {
        Self(Arc::new(AtomicF32::new(bpm.max(1.0))))
    }

    pub fn get(&self) -> f32 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, bpm: f32) {
        self.0.store(bpm.max(1.0), Ordering::Release);
    }
}

pub trait TimingSource: Clone {
    fn beats(&self) -> f32;
    fn bpm(&self) -> f32;
}

#[derive(Clone, Debug)]
pub enum Timing {
    Frame(FrameTiming),
    Osc(OscTransportTiming),
    Midi(MidiSongTiming),
    Hybrid(HybridTiming),
    Manual(ManualTiming),
}

impl Timing {
    pub fn frame(bpm: Bpm) -> Self {
        Self::Frame(FrameTiming::new(bpm))
    }

    pub fn osc(bpm: Bpm) -> Self {
        Self::Osc(OscTransportTiming::new(bpm))
    }

    pub fn midi(bpm: Bpm) -> Self {
        Self::Midi(MidiSongTiming::new(bpm))
    }

    pub fn hybrid(bpm: Bpm) -> Self {
        Self::Hybrid(HybridTiming::new(bpm))
    }

    pub fn manual(bpm: Bpm) -> Self {
        Self::Manual(ManualTiming::new(bpm))
    }

    pub fn set_external_beats(&self, beats: f32) {
        match self {
            Self::Osc(t) => t.set_beats(beats),
            Self::Midi(t) => t.set_beats(beats),
            Self::Hybrid(t) => t.set_beats(beats),
            Self::Manual(t) => t.set_beats(beats),
            Self::Frame(_) => {}
        }
    }
}

impl TimingSource for Timing {
    fn beats(&self) -> f32 {
        match self {
            Self::Frame(t) => t.beats(),
            Self::Osc(t) => t.beats(),
            Self::Midi(t) => t.beats(),
            Self::Hybrid(t) => t.beats(),
            Self::Manual(t) => t.beats(),
        }
    }

    fn bpm(&self) -> f32 {
        match self {
            Self::Frame(t) => t.bpm(),
            Self::Osc(t) => t.bpm(),
            Self::Midi(t) => t.bpm(),
            Self::Hybrid(t) => t.bpm(),
            Self::Manual(t) => t.bpm(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FrameTiming {
    bpm: Bpm,
}

impl FrameTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self { bpm }
    }
}

impl TimingSource for FrameTiming {
    fn beats(&self) -> f32 {
        let seconds_per_beat = 60.0 / self.bpm.get();
        let frames_per_beat = seconds_per_beat * frame_clock::fps();
        frame_clock::frame_count() as f32 / frames_per_beat.max(1.0)
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

#[derive(Clone, Debug)]
pub struct OscTransportTiming {
    bpm: Bpm,
    beats: Arc<AtomicF32>,
}

impl OscTransportTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self {
            bpm,
            beats: Arc::new(AtomicF32::new(0.0)),
        }
    }

    pub fn set_beats(&self, beats: f32) {
        self.beats.store(beats, Ordering::Release);
    }
}

impl TimingSource for OscTransportTiming {
    fn beats(&self) -> f32 {
        self.beats.load(Ordering::Acquire)
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

#[derive(Clone, Debug)]
pub struct MidiSongTiming {
    bpm: Bpm,
    beats: Arc<AtomicF32>,
}

impl MidiSongTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self {
            bpm,
            beats: Arc::new(AtomicF32::new(0.0)),
        }
    }

    pub fn set_beats(&self, beats: f32) {
        self.beats.store(beats, Ordering::Release);
    }
}

impl TimingSource for MidiSongTiming {
    fn beats(&self) -> f32 {
        self.beats.load(Ordering::Acquire)
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

#[derive(Clone, Debug)]
pub struct HybridTiming {
    bpm: Bpm,
    beats: Arc<AtomicF32>,
}

impl HybridTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self {
            bpm,
            beats: Arc::new(AtomicF32::new(0.0)),
        }
    }

    pub fn set_beats(&self, beats: f32) {
        self.beats.store(beats, Ordering::Release);
    }
}

impl TimingSource for HybridTiming {
    fn beats(&self) -> f32 {
        self.beats.load(Ordering::Acquire)
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

#[derive(Clone, Debug)]
pub struct ManualTiming {
    bpm: Bpm,
    beats: Arc<AtomicF32>,
}

impl ManualTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self {
            bpm,
            beats: Arc::new(AtomicF32::new(0.0)),
        }
    }

    pub fn set_beats(&self, beats: f32) {
        self.beats.store(beats, Ordering::Release);
    }
}

impl TimingSource for ManualTiming {
    fn beats(&self) -> f32 {
        self.beats.load(Ordering::Acquire)
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}
