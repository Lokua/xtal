//! Configuration for the default MIDI, OSC, and audio ports used in Lattice.
//! All ports can be overridden in the UI so there is no need to edit this file.

// pub const AUDIO_DEVICE_NAME: &str = "Lattice";
// pub const AUDIO_DEVICE_SAMPLE_RATE: usize = 48_000;
pub const MULTICHANNEL_AUDIO_DEVICE_NAME: &str = "Lattice16";
// pub const MULTICHANNEL_AUDIO_DEVICE_COUNT: usize = 16;
// pub const MULTICHANNEL_AUDIO_DEVICE_SAMPLE_RATE: usize = 48_000;
pub const MIDI_CLOCK_PORT: &str = "IAC Driver Lattice In";
pub const MIDI_CONTROL_IN_PORT: &str = "IAC Driver Lattice In";
pub const MIDI_CONTROL_OUT_PORT: &str = "IAC Driver Lattice In";
pub const OSC_PORT: u16 = 2346;
