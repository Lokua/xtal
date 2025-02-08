use cpal::{traits::*, Device, StreamConfig};
use nannou::math::map_range;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use super::prelude::*;

const CHANNEL_COUNT: usize = crate::config::MULTICHANNEL_AUDIO_DEVICE_COUNT;

struct AudioControlState {
    configs: HashMap<String, AudioControlConfig>,
    processor: MultichannelAudioProcessor,
    values: HashMap<String, f32>,
    previous_values: [f32; CHANNEL_COUNT],
}

pub struct AudioControls {
    pub is_active: bool,
    state: Arc<Mutex<AudioControlState>>,
}

impl AudioControls {
    pub fn new(fps: f32, sample_rate: usize) -> Self {
        let buffer_size = (sample_rate as f32 / fps).ceil() as usize;
        let processor = MultichannelAudioProcessor::new(buffer_size);
        Self {
            state: Arc::new(Mutex::new(AudioControlState {
                configs: HashMap::new(),
                values: HashMap::new(),
                processor,
                previous_values: [0.0; CHANNEL_COUNT],
            })),
            is_active: false,
        }
    }

    /// Add a new control. Overwrites any previous control of same name
    pub fn add(&mut self, name: &str, config: AudioControlConfig) {
        assert!(
            config.channel < CHANNEL_COUNT,
            "Channel must be less than {}",
            CHANNEL_COUNT
        );
        let mut state = self.state.lock().unwrap();
        state.configs.insert(name.to_string(), config);
    }

    /// Get the latest slewed and peak limited value by name,
    /// normalized to range [0, 1]. Returns 0.0 if name isn't found.
    pub fn get(&self, name: &str) -> f32 {
        self.state
            .lock()
            .unwrap()
            .values
            .get(name)
            .copied()
            .unwrap_or(0.0)
    }

    pub fn update_all_slew(&self, slew_config: SlewConfig) {
        let mut state = self.state.lock().unwrap();
        for (_, config) in state.configs.iter_mut() {
            config.slew_config = slew_config;
        }
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

                        let current_sample = state.processor.channel_value(
                            config.channel,
                            config.preemphasis,
                            config.detect,
                        );

                        let smoothed = state.processor.follow_envelope(
                            current_sample,
                            previous_sample,
                            config.slew_config,
                        );

                        let mapped = map_range(
                            smoothed,
                            0.0,
                            1.0,
                            config.range.0,
                            config.range.1,
                        );

                        (name.clone(), mapped, config.channel, smoothed)
                    })
                    .collect();

                for (name, mapped, channel, smoothed) in updates {
                    state.values.insert(name, mapped);
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

    pub fn control_from_config(
        mut self,
        name: &str,
        config: AudioControlConfig,
    ) -> Self {
        self.controls.add(&name, config);
        self
    }

    // TODO: control and control_mapped once we have settled on a more
    // permanent AudioControlConfig API

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
    pub preemphasis: f32,

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
        preemphasis: f32,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        Self {
            channel,
            slew_config,
            detect,
            preemphasis,
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

    fn channel_value(
        &self,
        channel: usize,
        preemphasis: f32,
        detection_mix: f32,
    ) -> f32 {
        let buffer = match self.channel_data.get(channel) {
            Some(buf) => buf,
            None => return 0.0,
        };

        let processed = if preemphasis > 0.0 {
            Self::apply_preemphasis(buffer, preemphasis)
        } else {
            buffer.to_vec()
        };

        MultichannelAudioProcessor::detect(&processed, detection_mix)
    }

    fn follow_envelope(
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

    /// Similar to follow_envelope but keeps audio in its original
    /// range of [-1, 1]
    #[allow(dead_code)]
    fn slew_limit(
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

    /// Standard preemphasis filter: `y[n] = x[n] - α * x[n-1]`
    /// 0.97 is common is it gives about +20dB emphasis starting around 1kHz
    pub fn apply_preemphasis(buffer: &[f32], coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(buffer.len());
        filtered.push(buffer[0]);

        for i in 1..buffer.len() {
            filtered.push(buffer[i] - coefficient * buffer[i - 1]);
        }

        filtered
    }

    /// Apply peak or RMS amplitude detection or mix between them,
    /// - 0.0 = only peak
    /// - 0.5 = 50/50 blend of peak and rms
    /// - 1.0 = only rms
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
