use cpal::traits::*;
use nannou::math::map_range;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use super::prelude::*;

const CV_DEVICE_NAME: &str = "Lattice16";
const N_CHANNELS: usize = 16;

#[derive(Clone, Debug)]
pub struct CvControlConfig {
    pub channel: usize,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl CvControlConfig {
    pub fn new(channel: usize, range: (f32, f32), default: f32) -> Self {
        let (min, max) = range;
        Self {
            channel,
            min,
            max,
            default,
        }
    }
}

#[derive(Default)]
pub struct CvState {
    values: HashMap<String, f32>,
}

impl CvState {
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

struct CvProcessor {
    channel_data: Vec<Vec<f32>>,
    buffer_size: usize,
}

impl CvProcessor {
    fn new(buffer_size: usize) -> Self {
        Self {
            channel_data: vec![Vec::with_capacity(buffer_size); N_CHANNELS],
            buffer_size,
        }
    }

    fn add_samples(&mut self, samples: &[f32]) {
        for (_, chunk) in samples.chunks(N_CHANNELS).enumerate() {
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

    fn get_channel_value(&self, channel: usize) -> f32 {
        self.channel_data
            .get(channel)
            .and_then(|data| data.last())
            .copied()
            .unwrap_or(0.0)
    }
}

pub struct CvControls {
    configs: HashMap<String, CvControlConfig>,
    state: Arc<Mutex<CvState>>,
    processor: Arc<Mutex<CvProcessor>>,
    is_active: bool,
}

impl CvControls {
    fn new(fps: f32, sample_rate: usize) -> Self {
        let buffer_size = (sample_rate as f32 / fps).ceil() as usize;
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(CvState::new())),
            processor: Arc::new(Mutex::new(CvProcessor::new(buffer_size))),
            is_active: false,
        }
    }

    fn add(&mut self, name: &str, config: CvControlConfig) {
        assert!(
            config.channel < N_CHANNELS,
            "Channel must be less than {}",
            N_CHANNELS
        );
        self.state.lock().unwrap().set(name, config.default);
        self.configs.insert(name.to_string(), config);
    }

    pub fn get(&self, name: &str) -> f32 {
        self.state.lock().unwrap().get(name)
    }

    fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
        let configs = self.configs.clone();
        let processor = self.processor.clone();

        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|d| d.name().map(|n| n == CV_DEVICE_NAME).unwrap_or(false))
            .expect("CV device not found");

        let config = device.default_input_config()?.into();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                let mut processor = processor.lock().unwrap();
                processor.add_samples(data);

                let mut state = state.lock().unwrap();
                for (name, config) in configs.iter() {
                    let value = processor.get_channel_value(config.channel);
                    let mapped =
                        map_range(value, -1.0, 1.0, config.min, config.max);
                    state.set(name, mapped);
                }
            },
            move |err| error!("Error in CV stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.is_active = true;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

pub struct CvControlBuilder {
    controls: CvControls,
}

impl CvControlBuilder {
    pub fn new(fps: f32) -> Self {
        let sample_rate = 48000;
        Self {
            controls: CvControls::new(fps, sample_rate),
        }
    }

    pub fn control_mapped(
        mut self,
        name: &str,
        channel: usize,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        let config = CvControlConfig::new(channel, range, default);
        self.controls.add(&name, config);
        self
    }

    pub fn control(self, name: &str, channel: usize, default: f32) -> Self {
        self.control_mapped(name, channel, (-1.0, 1.0), default)
    }

    pub fn build(mut self) -> CvControls {
        if let Err(e) = self.controls.start() {
            warn!(
                "Failed to initialize CV controls: {}. Using default values.",
                e
            );
        }
        self.controls
    }
}
