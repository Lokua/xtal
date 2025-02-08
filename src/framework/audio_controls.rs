use cpal::{traits::*, Device, StreamConfig};
use nannou::math::map_range;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use super::prelude::*;
use crate::config::MULTICHANNEL_AUDIO_DEVICE_COUNT;

const CHANNEL_COUNT: usize = MULTICHANNEL_AUDIO_DEVICE_COUNT;

struct AudioControlState {
    configs: HashMap<String, AudioControlConfig>,
    processor: MultichannelAudioProcessor,
    audio_state: AudioState,
    previous_values: [f32; CHANNEL_COUNT],
}

pub struct AudioControls {
    state: Arc<Mutex<AudioControlState>>,
    is_active: bool,
}

impl AudioControls {
    fn new(fps: f32, sample_rate: usize) -> Self {
        let buffer_size = (sample_rate as f32 / fps).ceil() as usize;
        let processor = MultichannelAudioProcessor::new(buffer_size);
        Self {
            state: Arc::new(Mutex::new(AudioControlState {
                configs: HashMap::new(),
                audio_state: AudioState::new(),
                processor,
                previous_values: [0.0; CHANNEL_COUNT],
            })),
            is_active: false,
        }
    }

    fn add(&mut self, name: &str, config: AudioControlConfig) {
        assert!(
            config.channel < CHANNEL_COUNT,
            "Channel must be less than {}",
            CHANNEL_COUNT
        );
        let mut state = self.state.lock().unwrap();
        state.configs.insert(name.to_string(), config);
    }

    pub fn get(&self, name: &str) -> f32 {
        self.state.lock().unwrap().audio_state.get(name)
    }

    pub fn update_all_slew(&self, slew_config: SlewConfig) {
        let mut state = self.state.lock().unwrap();
        for (_, config) in state.configs.iter_mut() {
            config.slew_config = slew_config;
        }
    }

    fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
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
                        let previous_sample =
                            state.previous_values[config.channel];

                        let current_sample =
                            state.processor.channel_value(config.channel);

                        let smoothed = state.processor.follow_envelope(
                            current_sample,
                            previous_sample,
                            config.slew_config,
                        );

                        let mapped = map_range(
                            smoothed, 0.0, 1.0, config.min, config.max,
                        );

                        (name.clone(), mapped, config.channel, smoothed)
                    })
                    .collect();

                for (name, mapped, channel, smoothed) in updates {
                    state.audio_state.set(&name, mapped);
                    state.previous_values[channel] = smoothed;
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

    pub fn is_active(&self) -> bool {
        self.is_active
    }
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
            ),
        }
    }

    pub fn control_mapped(
        mut self,
        name: &str,
        channel: usize,
        slew: (f32, f32),
        detect: f32,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        let config = AudioControlConfig::new(
            channel,
            SlewConfig::new(slew.0, slew.1),
            detect,
            range,
            default,
        );
        self.controls.add(&name, config);
        self
    }

    pub fn control(
        self,
        name: &str,
        channel: usize,
        slew: (f32, f32),
        detect: f32,
        default: f32,
    ) -> Self {
        self.control_mapped(name, channel, slew, detect, (-1.0, 1.0), default)
    }

    pub fn build(mut self) -> AudioControls {
        if let Err(e) = self.controls.start() {
            warn!(
                "Failed to initialize CV controls: {}. Using default values.",
                e
            );
        }
        self.controls
    }
}

#[derive(Clone, Debug)]
pub struct AudioControlConfig {
    /// The zero-indexed channel number (0 = first channel)
    pub channel: usize,
    pub slew_config: SlewConfig,

    /// Linearly interpolate between peak and RMS amplitude detection.
    /// 0.0 = peak, 1.0 = RMS
    pub detect: f32,

    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl AudioControlConfig {
    pub fn new(
        channel: usize,
        slew_config: SlewConfig,
        detect: f32,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        Self {
            channel,
            slew_config,
            detect,
            min: range.0,
            max: range.1,
            default,
        }
    }
}

#[derive(Debug)]
pub struct AudioState {
    values: HashMap<String, f32>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: f32) {
        self.values.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or(&0.0)
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

    fn channel_value(&self, channel: usize) -> f32 {
        self.channel_data
            .get(channel)
            .and_then(|data| data.last())
            .copied()
            .unwrap_or(0.0)
    }

    pub fn slew_limit(
        &self,
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

    pub fn follow_envelope(
        &self,
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
}
