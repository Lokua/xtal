//! Beat timing sources for Xtal's [`Animation`][animation] system.
//!
//! # Current Timing Implementations
//!
//! - Internal frame timing.
//! - External MIDI Clock with Song Position Pointer support.
//! - External Hybrid timing using MIDI Clock and MIDI Time Code.
//! - External OSC transport for Ableton Live via MaxForLive.
//! - Manual timing for static animation sequence visualization.
//!
//! The core abstraction is [`Timing`], which wraps the concrete
//! [`TimingSource`] implementations. In most sketches this is what you need
//! rather than the individual timing variants. Each source provides elapsed
//! musical time through [`TimingSource::beats`].
//!
//! When running a Xtal app, pass a timing mode after the sketch name to
//! override sketches that support the runtime timing source:
//!
//! ```sh
//! cargo run --release -- <sketch> osc
//! cargo run --release -- <sketch> midi
//! cargo run --release -- <sketch> hybrid
//! ```
//!
//! Available modes are `frame`, `osc`, `midi`, `hybrid`, and `manual`.
//!
//! [animation]: crate::motion

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use nannou_osc as osc;

use crate::core::prelude::*;
use crate::core::util::AtomicF32;
use crate::io::midi;
use crate::io::osc::SHARED_OSC_RECEIVER;
use crate::time::frame_clock;

const MIDI_START: u8 = 0xFA;
const MIDI_CLOCK: u8 = 0xF8;
const MIDI_SONG_POSITION: u8 = 0xF2;
const MIDI_MTC_QUARTER_FRAME: u8 = 0xF1;
const PULSES_PER_QUARTER_NOTE: u32 = 24;
const TICKS_PER_QUARTER_NOTE: u32 = 960;
const HYBRID_SYNC_THRESHOLD_BEATS: f32 = 0.5;

/// Shared, mutable BPM value used by timing sources.
///
/// Clones point to the same atomic value, so changing BPM through one handle
/// affects every timing source that was constructed from a clone of it.
#[derive(Clone, Debug)]
pub struct Bpm(Arc<AtomicF32>);

impl Bpm {
    /// Creates a BPM handle, clamping values below `1.0` to `1.0`.
    pub fn new(bpm: f32) -> Self {
        Self(Arc::new(AtomicF32::new(bpm.max(1.0))))
    }

    /// Returns the current BPM.
    pub fn get(&self) -> f32 {
        self.0.load(Ordering::Relaxed)
    }

    /// Updates the current BPM, clamping values below `1.0` to `1.0`.
    pub fn set(&self, bpm: f32) {
        self.0.store(bpm.max(1.0), Ordering::Release);
    }
}

/// Common interface for beat-based timing sources.
pub trait TimingSource: Clone {
    /// Returns the current transport position in beats.
    fn beats(&self) -> f32;
    /// Returns the current tempo in beats per minute.
    fn bpm(&self) -> f32;
}

/// Runtime-selectable timing source wrapper.
///
/// Use this enum when a sketch should accept the runtime timing mode instead
/// of committing to one concrete timing implementation.
#[derive(Clone, Debug)]
pub enum Timing {
    /// Beat position derived from Xtal's global frame clock.
    Frame(FrameTiming),
    /// Beat position received from OSC transport messages.
    Osc(OscTransportTiming),
    /// Beat position derived from MIDI Clock and Song Position Pointer.
    Midi(MidiSongTiming),
    /// Beat position derived from MIDI Clock with MIDI Time Code correction.
    Hybrid(HybridTiming),
    /// Beat position set explicitly by the caller.
    Manual(ManualTiming),
}

impl Timing {
    /// Creates frame-clock timing.
    pub fn frame(bpm: Bpm) -> Self {
        Self::Frame(FrameTiming::new(bpm))
    }

    /// Creates OSC transport timing.
    pub fn osc(bpm: Bpm) -> Self {
        Self::Osc(OscTransportTiming::new(bpm))
    }

    /// Creates MIDI song timing for a named MIDI clock port.
    pub fn midi(bpm: Bpm, port: &str) -> Self {
        Self::Midi(MidiSongTiming::new(bpm, port))
    }

    /// Creates hybrid MIDI timing for a named MIDI clock/MTC port.
    pub fn hybrid(bpm: Bpm, port: &str) -> Self {
        Self::Hybrid(HybridTiming::new(bpm, port))
    }

    /// Creates manual timing with an initial beat position of `0.0`.
    pub fn manual(bpm: Bpm) -> Self {
        Self::Manual(ManualTiming::new(bpm))
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

/// Timing source backed by Xtal's global frame clock.
#[derive(Clone, Debug)]
pub struct FrameTiming {
    bpm: Bpm,
}

impl FrameTiming {
    /// Creates frame-clock timing at the provided BPM.
    pub fn new(bpm: Bpm) -> Self {
        Self { bpm }
    }
}

impl TimingSource for FrameTiming {
    fn beats(&self) -> f32 {
        frame_clock::elapsed_seconds() * self.bpm.get() / 60.0
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

/// Timing source backed by OSC transport messages.
///
/// The shared OSC receiver is expected to deliver `/transport` messages with
/// playing state, bar, beat, and fractional tick values. Bar and beat values
/// are converted from one-based transport positions to zero-based beat counts.
#[derive(Clone, Debug)]
pub struct OscTransportTiming {
    bpm: Bpm,
    is_playing: Arc<AtomicBool>,
    bars: Arc<AtomicU32>,
    beats: Arc<AtomicU32>,
    ticks: Arc<AtomicU32>,
}

impl OscTransportTiming {
    /// Creates OSC transport timing and registers its listener callback.
    pub fn new(bpm: Bpm) -> Self {
        let timing = Self {
            bpm,
            is_playing: Arc::new(AtomicBool::new(false)),
            bars: Arc::new(AtomicU32::new(0)),
            beats: Arc::new(AtomicU32::new(0)),
            ticks: Arc::new(AtomicU32::new(0.0f32.to_bits())),
        };

        timing.setup_osc_listener();
        timing
    }

    fn setup_osc_listener(&self) {
        let is_playing = self.is_playing.clone();
        let bars = self.bars.clone();
        let beats = self.beats.clone();
        let ticks = self.ticks.clone();

        SHARED_OSC_RECEIVER.register_callback("/transport", move |msg| {
            if msg.args.len() < 4 {
                return;
            }

            if let (
                osc::Type::Int(a),
                osc::Type::Int(b),
                osc::Type::Int(c),
                osc::Type::Float(d),
            ) = (&msg.args[0], &msg.args[1], &msg.args[2], &msg.args[3])
            {
                is_playing.store(*a != 0, Ordering::Release);
                bars.store((*b).saturating_sub(1) as u32, Ordering::Release);
                beats.store((*c).saturating_sub(1) as u32, Ordering::Release);
                ticks.store(d.to_bits(), Ordering::Release);
            }
        });
    }
}

impl TimingSource for OscTransportTiming {
    fn beats(&self) -> f32 {
        if !self.is_playing.load(Ordering::Acquire) {
            return 0.0;
        }

        let bars = self.bars.load(Ordering::Acquire) as f32;
        let beats = self.beats.load(Ordering::Acquire) as f32;
        let ticks = f32::from_bits(self.ticks.load(Ordering::Acquire));
        (bars * 4.0) + beats + ticks
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

/// Timing source backed by MIDI Clock and Song Position Pointer.
///
/// Song Position Pointer establishes the coarse transport location, then MIDI
/// Clock pulses advance beat position from that base.
#[derive(Clone, Debug)]
pub struct MidiSongTiming {
    clock_count: Arc<AtomicU32>,
    song_position_ticks: Arc<AtomicU32>,
    bpm: Bpm,
}

impl MidiSongTiming {
    /// Creates MIDI song timing for `port`.
    ///
    /// If `port` is empty no MIDI listener is installed.
    pub fn new(bpm: Bpm, port: &str) -> Self {
        let timing = Self {
            clock_count: Arc::new(AtomicU32::new(0)),
            song_position_ticks: Arc::new(AtomicU32::new(0)),
            bpm,
        };

        timing.setup_midi_listener(port);
        timing
    }

    fn setup_midi_listener(&self, port: &str) {
        if port.is_empty() {
            info!("Skipping MIDI clock listener setup; no MIDI clock port.");
            return;
        }

        let clock_count = self.clock_count.clone();
        let song_position_ticks = self.song_position_ticks.clone();

        let result = midi::on_message(
            midi::ConnectionType::Clock,
            port,
            move |_stamp, message| {
                if message.is_empty() {
                    return;
                }

                match message[0] {
                    MIDI_CLOCK => {
                        clock_count.fetch_add(1, Ordering::SeqCst);
                    }
                    MIDI_SONG_POSITION => {
                        handle_song_position(message, &song_position_ticks);
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    MIDI_START => {
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    _ => {}
                }
            },
        );

        if let Err(err) = result {
            warn!(
                "Failed to initialize {:?} MIDI connection. Error: {}",
                midi::ConnectionType::Clock,
                err
            );
        }
    }
}

impl TimingSource for MidiSongTiming {
    fn beats(&self) -> f32 {
        let clock_offset = self.clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32;
        let beat_base = self.song_position_ticks.load(Ordering::Relaxed) as f32
            / TICKS_PER_QUARTER_NOTE as f32;
        beat_base + clock_offset
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

/// Timing source backed by MIDI Clock with MIDI Time Code resync.
///
/// MIDI Clock drives beat advancement. MIDI Time Code quarter-frame messages
/// are used to correct drift when the two positions differ by more than the
/// hybrid sync threshold.
#[derive(Clone, Debug)]
pub struct HybridTiming {
    clock_count: Arc<AtomicU32>,
    mtc_hours: Arc<AtomicU32>,
    mtc_minutes: Arc<AtomicU32>,
    mtc_seconds: Arc<AtomicU32>,
    mtc_frames: Arc<AtomicU32>,
    bpm: Bpm,
}

impl HybridTiming {
    /// Creates hybrid MIDI timing for `port`.
    ///
    /// If `port` is empty no MIDI listener is installed.
    pub fn new(bpm: Bpm, port: &str) -> Self {
        let timing = Self {
            clock_count: Arc::new(AtomicU32::new(0)),
            mtc_hours: Arc::new(AtomicU32::new(0)),
            mtc_minutes: Arc::new(AtomicU32::new(0)),
            mtc_seconds: Arc::new(AtomicU32::new(0)),
            mtc_frames: Arc::new(AtomicU32::new(0)),
            bpm,
        };

        timing.setup_midi_listener(port);
        timing
    }

    fn setup_midi_listener(&self, port: &str) {
        if port.is_empty() {
            info!("Skipping MIDI clock listener setup; no MIDI clock port.");
            return;
        }

        let clock_count = self.clock_count.clone();
        let mtc_hours = self.mtc_hours.clone();
        let mtc_minutes = self.mtc_minutes.clone();
        let mtc_seconds = self.mtc_seconds.clone();
        let mtc_frames = self.mtc_frames.clone();
        let bpm = self.bpm.clone();

        let result = midi::on_message(
            midi::ConnectionType::Clock,
            port,
            move |_stamp, message| {
                if message.is_empty() {
                    return;
                }

                match message[0] {
                    MIDI_CLOCK => {
                        clock_count.fetch_add(1, Ordering::SeqCst);
                    }
                    MIDI_START => {
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    MIDI_MTC_QUARTER_FRAME => {
                        handle_mtc_quarter_frame(
                            message,
                            &clock_count,
                            &mtc_hours,
                            &mtc_minutes,
                            &mtc_seconds,
                            &mtc_frames,
                            &bpm,
                        );
                    }
                    _ => {}
                }
            },
        );

        if let Err(err) = result {
            warn!(
                "Failed to initialize {:?} MIDI connection. Error: {}",
                midi::ConnectionType::Clock,
                err
            );
        }
    }
}

impl TimingSource for HybridTiming {
    fn beats(&self) -> f32 {
        self.clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32
    }

    fn bpm(&self) -> f32 {
        self.bpm.get()
    }
}

/// Timing source whose beat position is set explicitly by the caller.
///
/// This is useful for tests, static animation previews, and tooling that wants
/// to scrub musical time without a live transport.
#[derive(Clone, Debug)]
pub struct ManualTiming {
    bpm: Bpm,
    beats: Arc<AtomicF32>,
}

impl ManualTiming {
    /// Creates manual timing with beat position `0.0`.
    pub fn new(bpm: Bpm) -> Self {
        Self {
            bpm,
            beats: Arc::new(AtomicF32::new(0.0)),
        }
    }

    /// Sets the current beat position.
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

fn handle_song_position(message: &[u8], song_position_ticks: &AtomicU32) {
    if message.len() < 3 {
        warn!("Received malformed SONG_POSITION message: {:?}", message);
        return;
    }

    let lsb = message[1] as u32;
    let msb = message[2] as u32;
    let position = (msb << 7) | lsb;
    let tick_pos = position * (TICKS_PER_QUARTER_NOTE / 4);
    song_position_ticks.store(tick_pos, Ordering::SeqCst);
}

fn handle_mtc_quarter_frame(
    message: &[u8],
    clock_count: &AtomicU32,
    mtc_hours: &AtomicU32,
    mtc_minutes: &AtomicU32,
    mtc_seconds: &AtomicU32,
    mtc_frames: &AtomicU32,
    bpm: &Bpm,
) {
    if message.len() < 2 {
        return;
    }

    let data = message[1];
    let piece_index = (data >> 4) & 0x7;
    let value = data & 0xF;

    match piece_index {
        0 => set_low_nibble(mtc_frames, value),
        1 => set_high_nibble(mtc_frames, value),
        2 => set_low_nibble(mtc_seconds, value),
        3 => set_high_nibble(mtc_seconds, value),
        4 => set_low_nibble(mtc_minutes, value),
        5 => set_high_nibble(mtc_minutes, value),
        6 => set_low_nibble(mtc_hours, value),
        7 => sync_hybrid_from_mtc(
            value,
            clock_count,
            mtc_hours,
            mtc_minutes,
            mtc_seconds,
            mtc_frames,
            bpm,
        ),
        _ => {}
    }
}

fn set_low_nibble(target: &AtomicU32, value: u8) {
    let current = target.load(Ordering::Relaxed);
    target.store((current & 0xF0) | value as u32, Ordering::Relaxed);
}

fn set_high_nibble(target: &AtomicU32, value: u8) {
    let current = target.load(Ordering::Relaxed);
    target.store((current & 0x0F) | ((value as u32) << 4), Ordering::Relaxed);
}

fn sync_hybrid_from_mtc(
    value: u8,
    clock_count: &AtomicU32,
    mtc_hours: &AtomicU32,
    mtc_minutes: &AtomicU32,
    mtc_seconds: &AtomicU32,
    mtc_frames: &AtomicU32,
    bpm: &Bpm,
) {
    let hours_lsb = mtc_hours.load(Ordering::Relaxed) & 0x0F;
    let hours_msb = value & 0x3;
    let rate_code = (value >> 2) & 0x3;
    let fps = match rate_code {
        0 => 24.0,
        1 => 25.0,
        2 => 29.97,
        3 => 30.0,
        _ => return,
    };

    let full_hours = ((hours_msb << 4) | hours_lsb as u8) & 0x1F;
    mtc_hours.store(full_hours as u32, Ordering::Relaxed);

    let mtc_seconds_value = mtc_hours.load(Ordering::Relaxed) as f32 * 3600.0
        + mtc_minutes.load(Ordering::Relaxed) as f32 * 60.0
        + mtc_seconds.load(Ordering::Relaxed) as f32
        + mtc_frames.load(Ordering::Relaxed) as f32 / fps;
    let mtc_beats = mtc_seconds_value * (bpm.get() / 60.0);
    let midi_beats = clock_count.load(Ordering::Relaxed) as f32
        / PULSES_PER_QUARTER_NOTE as f32;
    let beat_difference = (mtc_beats - midi_beats).abs();

    if beat_difference > HYBRID_SYNC_THRESHOLD_BEATS {
        let clock = (mtc_beats * PULSES_PER_QUARTER_NOTE as f32) as u32;
        clock_count.store(clock, Ordering::SeqCst);
        trace!(
            "Hybrid timing resync from MTC: \
                mtc_beats={}, midi_beats={}, new_clock={}",
            mtc_beats, midi_beats, clock
        );
    }
}
