//! A timing abstraction built to allow various syncing mechanisms for Lattice's
//! [`Animation`][animation] system, including:
//!
//! - Internal frame counting
//! - External MIDI Clock with SPP support
//! - External MIDI Time Code
//! - External Hybrid mechanism utilizing both MIDI Clock and MTC for when SPP
//!   isn't supported
//! - External OSC for syncing specifically with Ableton Live via MaxForLive
//!   (preferred)
//! - Manual timing for generating visualizations of animation sequences
//!   statically
//!
//! The core abstraction is the [`Timing`] enum which wraps various
//! [`TimingSource`] implementations. In all cases you must provide a `bpm`
//! parameter as like the [`Animation`][animation] module, time is provided in
//! musical _beats_, e.g. 1.0 = 1 quarter note, 0.5 = 1 eight note, and so on.
//! This provides the most intuitive way to write animations that are in sync
//! with music.
//!
//! [animation]: lattice::framework::animation

use nannou_osc as osc;
use std::{
    env,
    error::Error,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use crate::framework::frame_controller;
use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::prelude::*;

#[derive(Clone, Debug)]
pub struct Bpm(Arc<AtomicF32>);

impl Bpm {
    pub fn new(bpm: f32) -> Self {
        Self(Arc::new(AtomicF32::new(bpm)))
    }

    pub fn get(&self) -> f32 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: f32) {
        self.0.store(value, Ordering::Release);
    }
}

pub trait TimingSource: Clone {
    fn beats(&self) -> f32;
    fn bpm(&self) -> f32;
}

/// Wrapper for all [`TimingSource`] implementations which allows
/// run-time selection of a `TimingSource` via command line argument.
/// Sketches can bypass the command line argument by using a `TimingSource`
/// other than this directly.
#[derive(Clone, Debug)]
pub enum Timing {
    Frame(FrameTiming),
    Osc(OscTransportTiming),
    Midi(MidiSongTiming),
    Hybrid(HybridTiming),
    Manual(ManualTiming),
}

impl Timing {
    /// Temporary constructor until we can port older sketch to Sketch trait
    pub fn new(bpm: Bpm) -> Self {
        let args: Vec<String> = env::args().collect();
        let timing_arg = args.get(2).map(|s| s.as_str()).unwrap_or("frame");
        let timing = match timing_arg {
            "osc" => Timing::Osc(OscTransportTiming::new(bpm)),
            "midi" => Timing::Midi(MidiSongTiming::new(bpm)),
            "hybrid" => Timing::Hybrid(HybridTiming::new(bpm)),
            _ => Timing::Frame(FrameTiming::new(bpm)),
        };
        info!("Using {} timing", timing_arg);
        timing
    }
}

impl TimingSource for Timing {
    fn bpm(&self) -> f32 {
        match self {
            Timing::Frame(t) => t.bpm(),
            Timing::Osc(t) => t.bpm(),
            Timing::Midi(t) => t.bpm(),
            Timing::Hybrid(t) => t.bpm(),
            Timing::Manual(t) => t.bpm(),
        }
    }

    fn beats(&self) -> f32 {
        match self {
            Timing::Frame(t) => t.beats(),
            Timing::Osc(t) => t.beats(),
            Timing::Midi(t) => t.beats(),
            Timing::Hybrid(t) => t.beats(),
            Timing::Manual(t) => t.beats(),
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
    fn bpm(&self) -> f32 {
        self.bpm.get()
    }

    fn beats(&self) -> f32 {
        let seconds_per_beat = 60.0 / self.bpm.get();
        let frames_per_beat = seconds_per_beat * frame_controller::fps();
        frame_controller::frame_count() as f32 / frames_per_beat
    }
}

pub const CLOCK: u8 = 0xF8; // 248
pub const START: u8 = 0xFA; // 250
pub const CONTINUE: u8 = 0xFB; // 251
pub const STOP: u8 = 0xFC; // 252
pub const SONG_POSITION: u8 = 0xF2; // 242

const PULSES_PER_QUARTER_NOTE: u32 = 24;
const TICKS_PER_QUARTER_NOTE: u32 = 960;

#[derive(Clone, Debug)]
pub struct MidiSongTiming {
    clock_count: Arc<AtomicU32>,

    /// When true, clock works as a subdivision of song_position; when false,
    /// clock is "absolute" and only reset on receive START. See `HybridTiming`
    /// for a combination of MTC and this struct for high precision sync for
    /// cases when SPP can't be relied on.
    follow_song_position_messages: bool,

    /// In MIDI ticks (1 tick = 1/960th of a quarter note)
    song_position: Arc<AtomicU32>,
    bpm: Bpm,
}

impl MidiSongTiming {
    pub fn new(bpm: Bpm) -> Self {
        let timing = Self {
            clock_count: Arc::new(AtomicU32::default()),
            follow_song_position_messages: true,
            song_position: Arc::new(AtomicU32::default()),
            bpm,
        };

        timing.setup_midi_listener();
        timing
    }

    /// Internal use for MtcTiming or HybridTiming
    pub fn new_no_song_position(bpm: Bpm) -> Self {
        let timing = Self {
            clock_count: Arc::new(AtomicU32::default()),
            follow_song_position_messages: false,
            song_position: Arc::new(AtomicU32::default()),
            bpm,
        };

        timing.setup_midi_listener();
        timing
    }

    fn setup_midi_listener(&self) {
        let clock_count = self.clock_count.clone();
        let song_position = self.song_position.clone();
        let follow_song_position_messages = self.follow_song_position_messages;

        match midi::on_message(
            midi::ConnectionType::Clock,
            crate::config::MIDI_CLOCK_PORT,
            move |_stamp, message| {
                if message.len() < 1 {
                    return;
                }
                match message[0] {
                    CLOCK => {
                        clock_count.fetch_add(1, Ordering::SeqCst);
                    }
                    SONG_POSITION => {
                        if !follow_song_position_messages {
                            debug!("Ignoring SPP");
                            return;
                        }

                        if message.len() < 3 {
                            warn!(
                                "Received malformed SONG_POSITION message: {:?}",
                                message
                            );
                        }

                        // Song position is a 14-bit value split across two bytes
                        let lsb = message[1] as u32;
                        let msb = message[2] as u32;
                        let position = ((msb << 7) | lsb) as u32;

                        debug!(
                            "Received SPP message: position={} (msb={}, lsb={})",
                            position, msb, lsb
                        );

                        let tick_pos = position * (TICKS_PER_QUARTER_NOTE / 4);
                        trace!("Converted to ticks: {}", tick_pos);

                        song_position.store(tick_pos, Ordering::SeqCst);

                        clock_count.store(0, Ordering::SeqCst);
                        trace!("Updated song position and reset clock count");
                    }
                    START => {
                        debug!("Received START message");
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    CONTINUE => {
                        debug!("Received CONTINUE message");
                    }
                    STOP => {
                        debug!("Received STOP message");
                    }
                    _ => {}
                }
            },
        ) {
            Ok(_) => {
                info!("MidiSongTiming initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize MidiSongTiming: {}. Using default values.", e);
            }
        }
    }

    fn beats(&self) -> f32 {
        let clock_offset = self.clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32;

        if self.follow_song_position_messages {
            let ticks = self.song_position.load(Ordering::Relaxed);
            let beats = ticks as f32 / TICKS_PER_QUARTER_NOTE as f32;

            beats + clock_offset
        } else {
            clock_offset
        }
    }
}

impl TimingSource for MidiSongTiming {
    fn bpm(&self) -> f32 {
        self.bpm.get()
    }

    fn beats(&self) -> f32 {
        self.beats()
    }
}

const MTC_QUARTER_FRAME: u8 = 0xF1;

#[derive(Clone, Debug)]
pub struct HybridTiming {
    midi_timing: MidiSongTiming,
    bpm: Bpm,

    // MTC time components - needed for tracking SMPTE position
    hours: Arc<AtomicU32>,
    minutes: Arc<AtomicU32>,
    seconds: Arc<AtomicU32>,
    frames: Arc<AtomicU32>,
}

impl HybridTiming {
    /// Sync when difference from MTC & MIDI Clock exceeds 1 beat
    const SYNC_THRESHOLD: f32 = 0.5;

    pub fn new(bpm: Bpm) -> Self {
        let timing = Self {
            midi_timing: MidiSongTiming::new_no_song_position(bpm.clone()),
            bpm,
            hours: Arc::new(AtomicU32::default()),
            minutes: Arc::new(AtomicU32::default()),
            seconds: Arc::new(AtomicU32::default()),
            frames: Arc::new(AtomicU32::default()),
        };

        timing.setup_mtc_listener();
        timing
    }

    fn setup_mtc_listener(&self) {
        let hours = self.hours.clone();
        let minutes = self.minutes.clone();
        let seconds = self.seconds.clone();
        let frames = self.frames.clone();
        let bpm = self.bpm.clone();
        let midi_timing = self.midi_timing.clone();

        match midi::on_message(
            midi::ConnectionType::Clock,
            crate::config::MIDI_CLOCK_PORT,
            move |_stamp, message| {
                if message.len() < 2 || message[0] != MTC_QUARTER_FRAME {
                    return;
                }

                let data = message[1];
                let piece_index = (data >> 4) & 0x7;
                let value = data & 0xF;

                match piece_index {
                    0 => {
                        let current = frames.load(Ordering::Relaxed);
                        frames.store(
                            (current & 0xF0) | value as u32,
                            Ordering::Relaxed,
                        );
                    }
                    1 => {
                        let current = frames.load(Ordering::Relaxed);
                        frames.store(
                            (current & 0x0F) | ((value as u32) << 4),
                            Ordering::Relaxed,
                        );
                    }
                    2 => {
                        let current = seconds.load(Ordering::Relaxed);
                        seconds.store(
                            (current & 0xF0) | value as u32,
                            Ordering::Relaxed,
                        );
                    }
                    3 => {
                        let current = seconds.load(Ordering::Relaxed);
                        seconds.store(
                            (current & 0x0F) | ((value as u32) << 4),
                            Ordering::Relaxed,
                        );
                    }
                    4 => {
                        let current = minutes.load(Ordering::Relaxed);
                        minutes.store(
                            (current & 0xF0) | value as u32,
                            Ordering::Relaxed,
                        );
                    }
                    5 => {
                        let current = minutes.load(Ordering::Relaxed);
                        minutes.store(
                            (current & 0x0F) | ((value as u32) << 4),
                            Ordering::Relaxed,
                        );
                    }
                    6 => {
                        let current = hours.load(Ordering::Relaxed);
                        hours.store(
                            (current & 0xF0) | value as u32,
                            Ordering::Relaxed,
                        );
                    }
                    7 => {
                        // Extract lower nibble (bits 0-3 of Type 6 quarter-frame message)
                        let hours_lsb = hours.load(Ordering::Relaxed) & 0x0F;

                        // Extract hours MSB (bits 0-1 of Type 7)
                        let hours_msb = value & 0x3;

                        // Extract frame rate code (bits 2-3 of Type 7)
                        let rate_code = (value >> 2) & 0x3;

                        // Convert rate code to fps
                        let fps = match rate_code {
                            0 => 24.0,
                            1 => 25.0,
                            // (drop-frame)
                            2 => 29.97,
                            3 => 30.0,
                            _ => unreachable!(),
                        };

                        let full_hours =
                            ((hours_msb << 4) | hours_lsb as u8) & 0x1F;

                        hours.store(full_hours as u32, Ordering::Relaxed);

                        let mtc_seconds = hours.load(Ordering::Relaxed) as f32
                            * 3600.0
                            + minutes.load(Ordering::Relaxed) as f32 * 60.0
                            + seconds.load(Ordering::Relaxed) as f32
                            + frames.load(Ordering::Relaxed) as f32 / fps;

                        let mtc_beats = mtc_seconds * (bpm.get() / 60.0);
                        let midi_beats = midi_timing.beats();

                        let beat_difference = (mtc_beats - midi_beats).abs();

                        if beat_difference > Self::SYNC_THRESHOLD {
                            let ticks = (mtc_beats
                                * TICKS_PER_QUARTER_NOTE as f32)
                                as u32;

                            midi_timing
                                .song_position
                                .store(ticks, Ordering::SeqCst);

                            let clock =
                                mtc_beats as u32 * PULSES_PER_QUARTER_NOTE;

                            midi_timing
                                .clock_count
                                .store(clock, Ordering::SeqCst);

                            debug!(
                                "Beat difference ({}) exceeds threshold. mtc_beats: {}, midi_beats: {}, resetting clock to: {}:",
                                beat_difference,
                                mtc_beats,
                                midi_beats,
                                clock
                            );

                            trace!("Synced MIDI position to {} ticks", ticks);
                        }
                    }
                    _ => {}
                }
            },
        ) {
            Ok(_) => {
                info!("HybridTiming initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize HybridTiming: {}", e);
            }
        }
    }

    fn beats(&self) -> f32 {
        self.midi_timing.beats()
    }
}

impl TimingSource for HybridTiming {
    fn bpm(&self) -> f32 {
        self.bpm.get()
    }

    fn beats(&self) -> f32 {
        self.beats()
    }
}

/// Uses the Open Sound Protocol to sync with Ableton Live via the
/// L.OSCTransport.amxd MaxForLive device included in this project's source code
/// on [GitHub][github]. See the project README's [OSC][osc] section for more
/// information.
///
/// [github]: https://github.com/Lokua/lattice
/// [osc]: https://github.com/Lokua/lattice#open-sound-control-osc
#[derive(Clone, Debug)]
pub struct OscTransportTiming {
    bpm: Bpm,
    is_playing: Arc<AtomicBool>,
    bars: Arc<AtomicU32>,
    beats: Arc<AtomicU32>,

    /// [0.0-1.0] fraction of beat that will be stored as bits
    /// to avoid losing precision
    ticks: Arc<AtomicU32>,
}

impl OscTransportTiming {
    pub fn new(bpm: Bpm) -> Self {
        let timing = Self {
            bpm,
            is_playing: Arc::new(AtomicBool::new(false)),
            bars: Arc::new(AtomicU32::default()),
            beats: Arc::new(AtomicU32::default()),
            ticks: Arc::new(AtomicU32::default()),
        };

        timing
            .setup_osc_listener()
            .expect("Unable to setup OSC listener");

        timing
    }

    fn setup_osc_listener(&self) -> Result<(), Box<dyn Error>> {
        let is_playing = self.is_playing.clone();
        let bars = self.bars.clone();
        let beats = self.beats.clone();
        let ticks = self.ticks.clone();

        SHARED_OSC_RECEIVER.register_callback("/transport", move |msg| match (
            &msg.args[0],
            &msg.args[1],
            &msg.args[2],
            &msg.args[3],
        ) {
            (
                osc::Type::Int(a),
                osc::Type::Int(b),
                osc::Type::Int(c),
                osc::Type::Float(d),
            ) => {
                is_playing.store(*a != 0, Ordering::Release);
                bars.store(*b as u32 - 1, Ordering::Release);
                beats.store(*c as u32 - 1, Ordering::Release);
                ticks.store(d.to_bits(), Ordering::Release);
            }
            _ => {}
        });

        Ok(())
    }

    fn beats(&self) -> f32 {
        if !self.is_playing.load(Ordering::Acquire) {
            return 0.0;
        }

        let bars = self.bars.load(Ordering::Acquire) as f32;
        let beats = self.beats.load(Ordering::Acquire) as f32;
        let ticks = f32::from_bits(self.ticks.load(Ordering::Acquire));

        (bars * 4.0) + beats + ticks
    }
}

impl TimingSource for OscTransportTiming {
    fn bpm(&self) -> f32 {
        self.bpm.get()
    }

    fn beats(&self) -> f32 {
        self.beats()
    }
}

/// Allows sketches to visualize animations statically by manually providing
/// what beat we're on. This is especially useful for visualizing
/// [`Breakpoint`] sequences
#[derive(Clone, Debug)]
pub struct ManualTiming {
    bpm: Bpm,
    beats: f32,
}

impl ManualTiming {
    pub fn new(bpm: Bpm) -> Self {
        Self { bpm, beats: 0.0 }
    }

    pub fn set_beats(&mut self, beats: f32) {
        self.beats = beats;
    }
}

impl TimingSource for ManualTiming {
    fn bpm(&self) -> f32 {
        self.bpm.get()
    }

    fn beats(&self) -> f32 {
        self.beats
    }
}

#[cfg(test)]
pub trait TestTiming {
    fn set_beats(&mut self, beat: f32);
}

#[cfg(test)]
impl TestTiming for OscTransportTiming {
    fn set_beats(&mut self, beat: f32) {
        let bars = (beat / 4.0).floor();
        let remaining_beats = beat - (bars * 4.0);
        let beats = remaining_beats.floor();
        let ticks = remaining_beats - beats;
        self.is_playing.store(true, Ordering::Release);
        self.bars.store(bars as u32, Ordering::Release);
        self.beats.store(beats as u32, Ordering::Release);
        self.ticks.store(ticks.to_bits(), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_midi_timing_beats() {
        let timing = MidiSongTiming::new(Bpm::new(120.0));

        // Simulate receiving SPP message for bar 44
        timing
            .song_position
            .store(44 * 4 * TICKS_PER_QUARTER_NOTE, Ordering::Relaxed);

        // Each bar is 4 beats, so bar 44 starts at beat 176
        assert_eq!(timing.beats(), 176.0);
    }
}
