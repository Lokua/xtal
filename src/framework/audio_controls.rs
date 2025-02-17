//! Control sketch parameters with audio signals. Supports any number of
//! channels of the device you can specify in
//! [`MULTICHANNEL_AUDIO_DEVICE_NAME`][device].
//!
//! [device]: crate::config::MULTICHANNEL_AUDIO_DEVICE_NAME

use cpal::{traits::*, Device, StreamConfig};
use nannou::math::map_range;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use super::frame_controller;
use super::prelude::*;

const CHANNEL_COUNT: usize = crate::config::MULTICHANNEL_AUDIO_DEVICE_COUNT;

/// A function used in [`AudioControls`] to reduce a channel's audio buffer to
/// a single value suitable for parameter control. The [`default_buffer_processor`]
/// is specifically for audio-rate signals, while [`thru_buffer_processor`]
/// should be used for control-rate audio signals (CV).
/// You can also pass you own custom processor.
pub type BufferProcessor =
    fn(buffer: &[f32], config: &AudioControlConfig) -> f32;

/// The default processor used in [`AudioControls`] that applies a pre-emphasis
/// filter to the incoming buffer then reduces that buffer into a single usable
/// value via peak or RMS amplitude detection.
pub fn default_buffer_processor(
    buffer: &[f32],
    config: &AudioControlConfig,
) -> f32 {
    let emphasized = MultichannelAudioProcessor::apply_pre_emphasis(
        buffer,
        config.pre_emphasis,
    );
    let smoothed =
        MultichannelAudioProcessor::detect(&emphasized, config.detect);

    smoothed
}

/// A "dummy" processor that simply returns that last value of the
/// audio buffer, leaving it unprocessed. Use this for CV.
pub fn thru_buffer_processor(
    buffer: &[f32],
    _config: &AudioControlConfig,
) -> f32 {
    *buffer.last().unwrap_or(&0.0)
}

pub struct AudioControlBuilder {
    controls: AudioControls,
}

impl AudioControlBuilder {
    pub fn new() -> Self {
        Self {
            controls: AudioControls::new(
                frame_controller::fps(),
                crate::config::MULTICHANNEL_AUDIO_DEVICE_SAMPLE_RATE,
                default_buffer_processor,
            ),
        }
    }

    /// See [`BufferProcessor`]
    pub fn with_buffer_processor(
        mut self,
        buffer_processor: BufferProcessor,
    ) -> Self {
        self.controls.buffer_processor = buffer_processor;
        self
    }

    /// See [`AudioControlConfig`]
    pub fn control_from_config(
        mut self,
        name: &str,
        config: AudioControlConfig,
    ) -> Self {
        self.controls.add(&name, config);
        self
    }

    pub fn control() -> Self {
        todo!()
    }

    pub fn control_mapped() -> Self {
        todo!()
    }

    pub fn build(mut self) -> AudioControls {
        if let Err(e) = self.controls.start() {
            error!(
                "Failed to initialize audio controls: {}. Using default values.",
                e
            );
        }
        self.controls
    }
}

struct AudioControlState {
    configs: HashMap<String, AudioControlConfig>,
    processor: MultichannelAudioProcessor,
    values: HashMap<String, f32>,
    previous_values: [f32; CHANNEL_COUNT],
}

pub struct AudioControls {
    pub is_active: bool,
    buffer_processor: BufferProcessor,
    state: Arc<Mutex<AudioControlState>>,
}

impl AudioControls {
    pub fn new(
        fps: f32,
        sample_rate: usize,
        buffer_processor: BufferProcessor,
    ) -> Self {
        let buffer_size = (sample_rate as f32 / fps).ceil() as usize;
        let processor = MultichannelAudioProcessor::new(buffer_size);
        Self {
            is_active: false,
            buffer_processor,
            state: Arc::new(Mutex::new(AudioControlState {
                configs: HashMap::new(),
                values: HashMap::new(),
                processor,
                previous_values: [0.0; CHANNEL_COUNT],
            })),
        }
    }

    /// Add a new control. Overwrites any previous control of the same name.
    pub fn add(&mut self, name: &str, config: AudioControlConfig) {
        let mut state = self.state.lock().unwrap();
        state.configs.insert(name.to_string(), config);
    }

    /// Get the latest processed audio value by name,
    /// normalized to [0, 1] then mapped to the range set in [`AudioControlConfig`].
    /// Returns 0.0 if name isn't found.
    pub fn get(&self, name: &str) -> f32 {
        self.state
            .lock()
            .unwrap()
            .values
            .get(name)
            .copied()
            .unwrap_or(0.0)
    }

    pub fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().values.contains_key(name)
    }

    pub fn update_control<F>(&mut self, name: &str, f: F)
    where
        F: FnOnce(&mut AudioControlConfig),
    {
        let mut state = self.state.lock().unwrap();
        if let Some(config) = state.configs.get_mut(name) {
            f(config);
        }
    }

    pub fn update_controls<F>(&mut self, f: F)
    where
        F: Fn(&mut AudioControlConfig),
    {
        let mut state = self.state.lock().unwrap();
        for config in state.configs.values_mut() {
            f(config);
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
        let buffer_processor = self.buffer_processor;
        let (device, stream_config) = Self::device_and_stream_config()?;

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &_| {
                let mut state = state.lock().unwrap();
                state.processor.add_samples(data);

                let updates: Vec<(String, f32, usize, f32)> = state
                    .configs
                    .iter()
                    .map(|(name, config)| {
                        let previous_value =
                            state.previous_values[config.channel];

                        let channel_buffer =
                            state.processor.channel_buffer(config.channel);

                        let processed_value =
                            buffer_processor(channel_buffer, &config);

                        let value = MultichannelAudioProcessor::follow_envelope(
                            processed_value,
                            previous_value,
                            config.slew_config,
                        );

                        let mapped = map_range(
                            value,
                            0.0,
                            1.0,
                            config.range.0,
                            config.range.1,
                        );

                        (name.clone(), mapped, config.channel, value)
                    })
                    .collect();

                for (name, mapped, channel, value) in updates {
                    state.values.insert(name, mapped);
                    state.previous_values[channel] = value;
                }
            },
            move |err| error!("Error in CV stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.is_active = true;

        info!(
            "AudioControls connected to device: {:?}",
            device.name().unwrap()
        );

        Ok(())
    }

    fn device_and_stream_config(
    ) -> Result<(Device, StreamConfig), Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|d| {
                d.name()
                    .map(|n| n == crate::config::MULTICHANNEL_AUDIO_DEVICE_NAME)
                    .unwrap_or(false)
            })
            .expect("CV device not found");

        let stream_config = device.default_input_config()?.into();

        Ok((device, stream_config))
    }
}

#[derive(Clone, Debug)]
pub struct AudioControlConfig {
    /// The zero-indexed channel number (0 = first channel)
    pub channel: usize,

    pub slew_config: SlewConfig,

    /// The pre-emphasis factor to apply to the audio signal.
    /// A higher value results in more emphasis on high frequencies.
    /// See [`MultichannelAudioProcessor::apply_pre_emphasis`] for more details
    pub pre_emphasis: f32,

    /// Linearly interpolate between peak and RMS amplitude detection.
    /// 0.0 = peak, 1.0 = RMS
    pub detect: f32,

    pub range: (f32, f32),
    pub default: f32,
}

impl AudioControlConfig {
    pub fn new(
        channel: usize,
        slew_config: SlewConfig,
        detect: f32,
        pre_emphasis: f32,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        Self {
            channel,
            slew_config,
            detect,
            pre_emphasis,
            range,
            default,
        }
    }
}

struct MultichannelAudioProcessor {
    channel_data: Vec<Vec<f32>>,
    buffer_size: usize,
}

impl MultichannelAudioProcessor {
    fn new(buffer_size: usize) -> Self {
        Self {
            channel_data: vec![Vec::with_capacity(buffer_size); CHANNEL_COUNT],
            buffer_size,
        }
    }

    fn add_samples(&mut self, samples: &[f32]) {
        for (_, chunk) in samples.chunks(CHANNEL_COUNT).enumerate() {
            for (channel, &sample) in chunk.iter().enumerate() {
                if let Some(channel_buffer) = self.channel_data.get_mut(channel)
                {
                    channel_buffer.push(sample);
                }
            }
        }

        // Ensure buffers are filled to their exact size
        for channel_buffer in &mut self.channel_data {
            if channel_buffer.len() > self.buffer_size {
                channel_buffer
                    .drain(0..(channel_buffer.len() - self.buffer_size));
            }
            while channel_buffer.len() < self.buffer_size {
                channel_buffer.push(0.0);
            }
        }
    }

    fn channel_buffer(&self, channel: usize) -> &[f32] {
        &self.channel_data[channel]
    }

    fn follow_envelope(
        sample: f32,
        previous_value: f32,
        slew_config: SlewConfig,
    ) -> f32 {
        let magnitude = sample.abs();

        let coeff = if magnitude > previous_value {
            1.0 - slew_config.rise
        } else {
            1.0 - slew_config.fall
        };

        previous_value + coeff * (magnitude - previous_value)
    }

    /// Similar to follow_envelope but keeps audio in its original
    /// range of [-1, 1]
    #[allow(dead_code)]
    fn slew_limit(
        sample: f32,
        previous_value: f32,
        slew_config: SlewConfig,
    ) -> f32 {
        let coeff = if sample > previous_value {
            1.0 - slew_config.rise
        } else {
            1.0 - slew_config.fall
        };

        previous_value + coeff * (sample - previous_value)
    }

    /// Standard pre-emphasis filter `y[n] = x[n] - α * x[n-1]` that amplifies high
    /// frequencies relative to low frequencies by subtracting a portion of the
    /// previous sample. It boosts high frequencies indirectly rather than
    /// explicitly cutting low frequencies with a sharp cutoff like a classical
    /// HPF. 0.97 is common as it gives about +20dB emphasis starting around 1kHz.
    /// See freq-response-preemphasis.png in the repo's assets folder for more
    /// details.
    pub fn apply_pre_emphasis(buffer: &[f32], coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(buffer.len());
        filtered.push(buffer[0]);

        for i in 1..buffer.len() {
            filtered.push(buffer[i] - coefficient * buffer[i - 1]);
        }

        filtered
    }

    /// Linearly interpolate between peak and RMS amplitude detection.
    /// 0.0 = peak, 1.0 = RMS
    fn detect(buffer: &[f32], method_mix: f32) -> f32 {
        if method_mix == 0.0 {
            return Self::peak(buffer);
        }
        if method_mix == 1.0 {
            return Self::rms(buffer);
        }
        let peak = Self::peak(buffer);
        let rms = Self::rms(buffer);

        return (peak * method_mix) + (rms * (1.0 - method_mix));
    }

    fn peak(buffer: &[f32]) -> f32 {
        buffer.iter().fold(f32::MIN, |a, &b| f32::max(a, b))
    }

    fn rms(buffer: &[f32]) -> f32 {
        (buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32)
            .sqrt()
    }
}
