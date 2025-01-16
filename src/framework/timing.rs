use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::framework::prelude::*;

// Core trait that defines what we need from any timing source
pub trait TimingSource: Clone {
    /// Get the current beat position (fractional beats from start)
    fn beat_position(&self) -> f32;

    /// Get total number of beats elapsed
    fn total_beats(&self) -> f32;

    /// Convert beats to frames (for compatibility with frame-based code)
    fn beats_to_frames(&self, beats: f32) -> f32;
}

// The original frame-based implementation
#[derive(Clone)]
pub struct FrameTiming {
    bpm: f32,
    fps: f32,
}

impl FrameTiming {
    pub fn new(bpm: f32) -> Self {
        Self {
            bpm,
            fps: frame_controller::fps(),
        }
    }

    fn current_frame(&self) -> f32 {
        frame_controller::frame_count() as f32
    }
}

impl TimingSource for FrameTiming {
    fn beat_position(&self) -> f32 {
        self.current_frame() / self.beats_to_frames(1.0)
    }

    fn total_beats(&self) -> f32 {
        self.beat_position()
    }

    fn beats_to_frames(&self, beats: f32) -> f32 {
        let seconds_per_beat = 60.0 / self.bpm;
        let total_seconds = beats * seconds_per_beat;
        total_seconds * self.fps
    }
}

pub const TIMING_CLOCK: u8 = 248;
pub const START: u8 = 250;
pub const STOP: u8 = 252;
pub const SONG_POSITION: u8 = 242;
const PULSES_PER_QUARTER_NOTE: u32 = 24;
const TICKS_PER_QUARTER_NOTE: u32 = 960;

// MIDI Song Position timing implementation
#[derive(Clone)]
pub struct MidiSongTiming {
    // Atomic counters for thread safety
    clock_count: Arc<AtomicU32>,
    // In MIDI ticks (1 tick = 1/960th of a quarter note)
    song_position: Arc<AtomicU32>,
    bpm: f32,
}

impl MidiSongTiming {
    pub fn new(bpm: f32) -> Self {
        let timing = Self {
            clock_count: Arc::new(AtomicU32::default()),
            song_position: Arc::new(AtomicU32::default()),
            bpm,
        };

        timing.setup_midi_listener();
        timing
    }

    fn setup_midi_listener(&self) {
        let clock_count = self.clock_count.clone();
        let song_position = self.song_position.clone();

        match on_message(
            move |message| {
                if message.len() < 1 {
                    return;
                }
                match message[0] {
                    TIMING_CLOCK => {
                        clock_count.fetch_add(1, Ordering::SeqCst);
                    }
                    SONG_POSITION => {
                        if message.len() < 3 {
                            return;
                        }
                        // Song position is a 14-bit value split across two bytes
                        let lsb = message[1] as u32;
                        let msb = message[2] as u32;
                        let position = ((msb << 7) | lsb) as u32;

                        // Convert from MIDI beats (16th notes) to our tick resolution
                        // 1 MIDI beat = 6 MIDI clock pulses
                        let tick_pos = position * 6;
                        song_position.store(tick_pos, Ordering::SeqCst);
                        // Reset clock count when position changes
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    START => {
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    STOP => {}
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
        // Convert MIDI ticks to beats (quarter notes)
        let ticks = self.song_position.load(Ordering::Relaxed);
        let beats = ticks as f32 / TICKS_PER_QUARTER_NOTE as f32;

        // Add fractional position from clock count
        let clock_offset = self.clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32;

        beats + clock_offset
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

// Helper functions for testing
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_frame_timing_conversion() {
        let timing = FrameTiming::new(120.0);

        // At 120 BPM, one beat = 0.5 seconds
        // At 60 FPS, 0.5 seconds = 30 frames
        assert_eq!(timing.beats_to_frames(1.0), 30.0);
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
