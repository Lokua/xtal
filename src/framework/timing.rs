use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::framework::prelude::*;

pub trait TimingSource: Clone {
    fn beat_position(&self) -> f32;
    fn total_beats(&self) -> f32;
    fn beats_to_frames(&self, beats: f32) -> f32;
}

#[derive(Clone)]
pub struct FrameTiming {
    bpm: f32,
}

impl FrameTiming {
    pub fn new(bpm: f32) -> Self {
        Self { bpm }
    }
}

impl TimingSource for FrameTiming {
    fn beat_position(&self) -> f32 {
        frame_controller::frame_count() as f32 / self.beats_to_frames(1.0)
    }

    fn total_beats(&self) -> f32 {
        self.beat_position()
    }

    fn beats_to_frames(&self, beats: f32) -> f32 {
        let seconds_per_beat = 60.0 / self.bpm;
        let total_seconds = beats * seconds_per_beat;
        total_seconds * frame_controller::fps()
    }
}

pub const CLOCK: u8 = 0xF8; // 248
pub const START: u8 = 0xFA; // 250
pub const CONTINUE: u8 = 0xFB; // 251
pub const STOP: u8 = 0xFC; // 252
pub const SONG_POSITION: u8 = 0xF2; // 242

const PULSES_PER_QUARTER_NOTE: u32 = 24;
const TICKS_PER_QUARTER_NOTE: u32 = 960;

#[derive(Clone)]
pub struct MidiSongTiming {
    clock_count: Arc<AtomicU32>,

    /// When true, clock works as a subdivision of song_position;
    /// when false, clock is "absolute" and only reset on receive START.
    /// See `HybridTiming` for a combination of MTC and this struct for
    /// high precision sync for cases when SPP can't be relied on.
    follow_song_position_messages: bool,

    /// In MIDI ticks (1 tick = 1/960th of a quarter note)
    song_position: Arc<AtomicU32>,
    bpm: f32,
}

impl MidiSongTiming {
    pub fn new(bpm: f32) -> Self {
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
    pub fn new_no_song_position(bpm: f32) -> Self {
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

        match on_message(
            move |message| {
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

                        trace!(
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
            "[MidiSongTiming]",
        ) {
            Ok(_) => {
                info!("MidiSongTiming initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize MidiSongTiming: {}. Using default values.", e);
            }
        }
    }

    fn get_position_in_beats(&self) -> f32 {
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
    fn beat_position(&self) -> f32 {
        self.get_position_in_beats()
    }

    fn total_beats(&self) -> f32 {
        self.get_position_in_beats()
    }

    fn beats_to_frames(&self, beats: f32) -> f32 {
        let bpm = self.bpm;
        let seconds_per_beat = 60.0 / bpm;
        beats * seconds_per_beat * frame_controller::fps()
    }
}

const MTC_QUARTER_FRAME: u8 = 0xF1;

#[derive(Clone)]
pub struct HybridTiming {
    midi_timing: MidiSongTiming,
    bpm: f32,

    // MTC time components - needed for tracking SMPTE position
    hours: Arc<AtomicU32>,
    minutes: Arc<AtomicU32>,
    seconds: Arc<AtomicU32>,
    frames: Arc<AtomicU32>,
}

impl HybridTiming {
    /// Sync when difference from MTC & MIDI Clock exceeds 1 beat
    const BEAT_SYNC_THRESHOLD: f32 = 0.5;

    pub fn new(bpm: f32) -> Self {
        let timing = Self {
            midi_timing: MidiSongTiming::new_no_song_position(bpm),
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
        let bpm = self.bpm;
        let midi_timing = self.midi_timing.clone();

        match on_message(
            move |message| {
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
                            0 => 24.0,  // 24 fps
                            1 => 25.0,  // 25 fps
                            2 => 29.97, // 29.97 fps (drop-frame)
                            3 => 30.0,  // 30 fps (non-drop frame)
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

                        let mtc_beats = mtc_seconds * (bpm / 60.0);
                        let midi_beats = midi_timing.beat_position();

                        let beat_difference = (mtc_beats - midi_beats).abs();

                        if beat_difference > Self::BEAT_SYNC_THRESHOLD {
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
            "[HybridTiming]",
        ) {
            Ok(_) => {
                info!("HybridTiming MTC listener initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize HybridTiming MTC listener: {}", e);
            }
        }
    }

    fn get_position_in_beats(&self) -> f32 {
        self.midi_timing.beat_position()
    }
}

impl TimingSource for HybridTiming {
    fn beat_position(&self) -> f32 {
        self.get_position_in_beats()
    }

    // TODO: none of our implementations need this.
    // Also, rename `beat_position` to beats for clarity
    fn total_beats(&self) -> f32 {
        self.beat_position()
    }

    fn beats_to_frames(&self, beats: f32) -> f32 {
        self.midi_timing.beats_to_frames(beats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_frame_timing_conversion() {
        let timing = FrameTiming::new(120.0);

        // At 120 BPM, one beat = 0.5 seconds
        // At 24 FPS, 0.5 seconds = 12 frames
        assert_eq!(timing.beats_to_frames(1.0), 12.0);
    }

    #[test]
    #[serial]
    fn test_midi_timing_beats() {
        let timing = MidiSongTiming::new(120.0);

        // Simulate receiving SPP message for bar 44
        timing
            .song_position
            .store(44 * 4 * TICKS_PER_QUARTER_NOTE, Ordering::Relaxed);

        // Each bar is 4 beats, so bar 44 starts at beat 176
        assert_eq!(timing.beat_position(), 176.0);
    }
}
