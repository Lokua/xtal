use cpal::{Device, Stream, StreamConfig, traits::*};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::control_traits::{ControlCollection, ControlConfig};
use crate::framework::frame_controller;
use crate::framework::prelude::*;
use crate::motion::SlewLimiter;
use crate::warn_once;

#[derive(Clone, Debug)]
pub struct AudioControlConfig {
    pub channel: usize,
    pub slew_limiter: SlewLimiter,
    pub pre_emphasis: f32,
    pub detect: f32,
    pub range: (f32, f32),
    pub value: f32,
}

impl AudioControlConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: usize,
        slew_limiter: SlewLimiter,
        detect: f32,
        pre_emphasis: f32,
        range: (f32, f32),
        value: f32,
    ) -> Self {
        Self {
            channel,
            slew_limiter,
            pre_emphasis,
            detect,
            range,
            value,
        }
    }
}

impl ControlConfig<f32, f32> for AudioControlConfig {}

#[derive(Debug)]
struct State {
    configs: HashMap<String, AudioControlConfig>,
    processor: MultichannelAudioProcessor,
    values: HashMap<String, f32>,
}

pub type BufferProcessor =
    fn(buffer: &[f32], config: &AudioControlConfig) -> f32;

pub fn default_buffer_processor(
    buffer: &[f32],
    config: &AudioControlConfig,
) -> f32 {
    MultichannelAudioProcessor::detect(
        &MultichannelAudioProcessor::apply_pre_emphasis(
            buffer,
            config.pre_emphasis,
        ),
        config.detect,
    )
}

pub fn thru_buffer_processor(
    buffer: &[f32],
    _config: &AudioControlConfig,
) -> f32 {
    *buffer.last().unwrap_or(&0.0)
}

#[derive(Clone)]
pub struct AudioControls {
    pub is_active: bool,
    buffer_processor: BufferProcessor,
    state: Arc<Mutex<State>>,
    device_name: Option<String>,
    stream: Option<Arc<Stream>>,
}

impl Default for AudioControls {
    fn default() -> Self {
        Self::new(default_buffer_processor)
    }
}

impl AudioControls {
    pub fn new(buffer_processor: BufferProcessor) -> Self {
        let processor = MultichannelAudioProcessor::new(800, 16);
        Self {
            is_active: false,
            buffer_processor,
            state: Arc::new(Mutex::new(State {
                configs: HashMap::default(),
                processor,
                values: HashMap::default(),
            })),
            device_name: None,
            stream: None,
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

    pub fn set_buffer_processor(&mut self, buffer_processor: BufferProcessor) {
        self.buffer_processor = buffer_processor
    }

    pub fn set_device_name(&mut self, device_name: String) {
        self.device_name = if device_name.is_empty() {
            None
        } else {
            Some(device_name)
        };
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(device_name) = self.device_name.clone() else {
            warn!("Skipping AudioControls listener setup; no audio device.");
            self.is_active = false;
            return Ok(());
        };

        let buffer_processor = self.buffer_processor;
        let (device, stream_config) =
            Self::device_and_stream_config(&device_name)?;

        {
            let mut state = self.state.lock().unwrap();
            let buffer_size =
                stream_config.sample_rate.0 as f32 / frame_controller::fps();
            let buffer_size = buffer_size.ceil() as usize;
            let channels = stream_config.channels as usize;
            state.processor =
                MultichannelAudioProcessor::new(buffer_size, channels);
        }

        let state = self.state.clone();
        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &_| {
                let mut state = state.lock().unwrap();
                state.processor.add_samples(data);

                let updates: Vec<(String, f32)> = state
                    .configs
                    .iter()
                    .filter_map(|(name, config)| {
                        if config.channel >= state.processor.channel_data.len()
                        {
                            warn_once!(
                                "Using AudioControlConfig with channel beyond available device channels: {:?}",
                                config
                            );
                            return None;
                        }

                        let channel_buffer =
                            state.processor.channel_buffer(config.channel);
                        let processed_value =
                            buffer_processor(channel_buffer, config);
                        let value = config.slew_limiter.apply(processed_value);
                        let mapped = map_range(
                            value,
                            0.0,
                            1.0,
                            config.range.0,
                            config.range.1,
                        );
                        Some((name.clone(), mapped))
                    })
                    .collect();

                for (name, mapped) in updates {
                    state.values.insert(name, mapped);
                }
            },
            move |err| error!("Error in audio stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(Arc::new(stream));
        self.is_active = true;
        info!("AudioControls connected to device: {:?}", device.name()?);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(_stream) = self.stream.take() {
            self.is_active = false;
            debug!("Audio stream stopped");
        }
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop();
        info!("Restarting...");
        thread::sleep(Duration::from_millis(10));
        self.start()
    }

    fn device_and_stream_config(
        device_name: &str,
    ) -> Result<(Device, StreamConfig), Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| {
                Box::<dyn Error>::from(format!(
                    "Audio device '{}' not found",
                    device_name
                ))
            })?;

        let stream_config = device.default_input_config()?.into();
        Ok((device, stream_config))
    }
}

impl
    ControlCollection<
        AudioControlConfig,
        f32,
        f32,
        HashMap<String, AudioControlConfig>,
    > for AudioControls
{
    fn add(&mut self, name: &str, config: AudioControlConfig) {
        let mut state = self.state.lock().unwrap();
        state.values.insert(name.to_string(), config.value);
        state.configs.insert(name.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<AudioControlConfig> {
        self.state.lock().unwrap().configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, AudioControlConfig> {
        self.state.lock().unwrap().configs.clone()
    }

    fn get(&self, name: &str) -> f32 {
        self.get_optional(name).unwrap_or(0.0)
    }

    fn get_optional(&self, name: &str) -> Option<f32> {
        self.state.lock().unwrap().values.get(name).copied()
    }

    fn remove(&mut self, name: &str) {
        let mut state = self.state.lock().unwrap();
        state.configs.remove(name);
        state.values.remove(name);
    }

    fn set(&mut self, name: &str, value: f32) {
        self.state
            .lock()
            .unwrap()
            .values
            .insert(name.to_string(), value);
    }

    fn values(&self) -> HashMap<String, f32> {
        self.state.lock().unwrap().values.clone()
    }

    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, f32>),
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state.values);
    }
}

impl std::fmt::Debug for AudioControls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioControls")
            .field("is_active", &self.is_active)
            .field("buffer_processor", &"<function pointer>")
            .field("state", &self.state)
            .field("device_name", &self.device_name)
            .field(
                "stream",
                &if self.stream.is_some() {
                    "Some(Stream)"
                } else {
                    "None"
                },
            )
            .finish()
    }
}

#[derive(Default)]
pub struct AudioControlBuilder {
    controls: AudioControls,
}

impl AudioControlBuilder {
    pub fn new() -> Self {
        Self {
            controls: AudioControls::new(default_buffer_processor),
        }
    }

    pub fn control(mut self, name: &str, config: AudioControlConfig) -> Self {
        self.controls.add(name, config);
        self
    }

    pub fn with_buffer_processor(
        mut self,
        buffer_processor: BufferProcessor,
    ) -> Self {
        self.controls.buffer_processor = buffer_processor;
        self
    }

    pub fn build(mut self) -> AudioControls {
        if let Err(e) = self.controls.start() {
            warn!(
                "Failed to initialize audio controls: {}. Using default values.",
                e
            );
        }
        self.controls
    }
}

#[derive(Debug)]
struct MultichannelAudioProcessor {
    channel_data: Vec<Vec<f32>>,
    buffer_size: usize,
}

impl MultichannelAudioProcessor {
    fn new(buffer_size: usize, channel_count: usize) -> Self {
        Self {
            channel_data: vec![vec![0.0; buffer_size]; channel_count],
            buffer_size,
        }
    }

    fn add_samples(&mut self, samples: &[f32]) {
        for chunk in samples.chunks(self.channel_data.len()) {
            for (channel, &sample) in chunk.iter().enumerate() {
                if let Some(buffer) = self.channel_data.get_mut(channel) {
                    buffer.push(sample);
                }
            }
        }

        for buffer in &mut self.channel_data {
            if buffer.len() > self.buffer_size {
                buffer.drain(0..(buffer.len() - self.buffer_size));
            }
            while buffer.len() < self.buffer_size {
                buffer.push(0.0);
            }
        }
    }

    fn channel_buffer(&self, channel: usize) -> &[f32] {
        &self.channel_data[channel]
    }

    pub fn apply_pre_emphasis(buffer: &[f32], coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(buffer.len());
        filtered.push(*buffer.first().unwrap_or(&0.0));
        for i in 1..buffer.len() {
            filtered.push(buffer[i] - coefficient * buffer[i - 1]);
        }
        filtered
    }

    fn detect(buffer: &[f32], method_mix: f32) -> f32 {
        if method_mix == 0.0 {
            return Self::peak(buffer);
        }
        if method_mix == 1.0 {
            return Self::rms(buffer);
        }
        let peak = Self::peak(buffer);
        let rms = Self::rms(buffer);
        (peak * method_mix) + (rms * (1.0 - method_mix))
    }

    fn peak(buffer: &[f32]) -> f32 {
        buffer.iter().fold(f32::MIN, |a, &b| f32::max(a, b))
    }

    fn rms(buffer: &[f32]) -> f32 {
        (buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32)
            .sqrt()
    }
}
