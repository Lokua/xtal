pub const OSC_PORT: u16 = 2346;

/// The audio device used for single-channel, multiband audio processing.
/// Uses the device's 1st channel (channel 0) for processing.
/// See `framework::audio`
pub const AUDIO_DEVICE_NAME: &str = "Lattice";
pub const AUDIO_DEVICE_SAMPLE_RATE: usize = 48_000;

/// The audio device used for control-rate audio processing
pub const CV_DEVICE_NAME: &str = "Lattice16";
/// The number of channels we will attempt to process for CV.
/// Assumes channels start from 0.
pub const CV_DEVICE_CHANNEL_COUNT: usize = 16;
pub const CV_DEVICE_SAMPLE_RATE: usize = 48_000;

/// The name of the MIDI device/port that will be used for control data
/// processing and clock.
/// TODO: separate clock and control ports
pub const MIDI_INPUT_PORT: &str = "IAC Driver Lattice In";
