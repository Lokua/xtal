//! Control sketch parameters with audio signals.
//!
//! Supports any number of channels of the device you can specify in
//! [`MULTICHANNEL_AUDIO_DEVICE_NAME`][device]. Note that sketches do not need
//! to interact with this module directly - see [`ControlHub`].
//!
//! [device]: crate::config::MULTICHANNEL_AUDIO_DEVICE_NAME
use cpal::{traits::*, Device, Stream, StreamConfig};
use nannou::math::map_range;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::framework::frame_controller;
use crate::framework::prelude::*;

#[derive(Clone, Debug)]
pub struct AudioControlConfig {
    /// The zero-indexed channel number (0 = first channel)
    pub channel: usize,

    pub slew_limiter: SlewLimiter,

    #[allow(rustdoc::private_intra_doc_links)]
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
        slew_limiter: SlewLimiter,
        detect: f32,
        pre_emphasis: f32,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        Self {
            channel,
            slew_limiter,
            detect,
            pre_emphasis,
            range,
            default,
        }
    }
}

/// A function used in [`AudioControls`] to reduce a channel's audio buffer to a
/// single value suitable for parameter control. The
/// [`default_buffer_processor`] is specifically for audio-rate signals, while
/// [`thru_buffer_processor`] should be used for control-rate audio signals
/// (CV). You can also pass you own custom processor.
pub type BufferProcessor =
    fn(buffer: &[f32], config: &AudioControlConfig) -> f32;

/// The default processor used in [`AudioControls`] that applies a pre-emphasis
/// filter to the incoming buffer then reduces that buffer into a single usable
/// value via peak or RMS amplitude detection.
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

/// A "dummy" processor that simply returns that last value of the
/// audio buffer, leaving it unprocessed. Use this for CV.
pub fn thru_buffer_processor(
    buffer: &[f32],
    _config: &AudioControlConfig,
) -> f32 {
    *buffer.last().unwrap_or(&0.0)
}

#[derive(Debug)]
struct AudioControlState {
    configs: HashMap<String, AudioControlConfig>,
    processor: MultichannelAudioProcessor,
    values: HashMap<String, f32>,
    previous_values: Vec<f32>,
}

pub struct AudioControls {
    pub is_active: bool,
    buffer_processor: BufferProcessor,
    state: Arc<Mutex<AudioControlState>>,
    stream: Option<Stream>,
}

impl std::fmt::Debug for AudioControls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioControls")
            .field("is_active", &self.is_active)
            .field("buffer_processor", &"<function pointer>")
            .field("state", &self.state)
            .field(
                "stream",
                &ternary!(self.stream.is_some(), "Some(Stream)", "None"),
            )
            .finish()
    }
}

impl AudioControls {
    pub fn new(buffer_processor: BufferProcessor) -> Self {
        // TODO: refactor - none of this data is needed until we call `start`
        let processor = MultichannelAudioProcessor::new(800, 16);
        Self {
            is_active: false,
            buffer_processor,
            state: Arc::new(Mutex::new(AudioControlState {
                configs: HashMap::default(),
                values: HashMap::default(),
                processor,
                previous_values: vec![0.0],
            })),
            stream: None,
        }
    }

    /// Add a new control. Overwrites any previous control of the same name.
    pub fn add(&mut self, name: &str, config: AudioControlConfig) {
        let mut state = self.state.lock().unwrap();
        state.values.insert(name.to_string(), config.default);
        state.configs.insert(name.to_string(), config);
    }

    /// Get the latest processed audio value by name, normalized to [0, 1] then
    /// mapped to the range set in [`AudioControlConfig`]. Returns 0.0 if name
    /// isn't found.
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

    pub fn set_buffer_processor(&mut self, buffer_processor: BufferProcessor) {
        self.buffer_processor = buffer_processor
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let buffer_processor = self.buffer_processor;
        let (device, stream_config) = Self::device_and_stream_config()?;

        {
            let mut state = self.state.lock().unwrap();
            let buffer_size =
                stream_config.sample_rate.0 as f32 / frame_controller::fps();
            let buffer_size = buffer_size.ceil() as usize;
            let channels = stream_config.channels as usize;
            state.processor =
                MultichannelAudioProcessor::new(buffer_size, channels);
            state.previous_values = vec![0.0; channels];
        }

        let state = self.state.clone();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &_| {
                let mut state = state.lock().unwrap();
                state.processor.add_samples(data);

                let updates: Vec<(String, f32, usize, f32)> = state
                    .configs
                    .iter()
                    .filter_map(|(name, config)| {
                        if config.channel >= state.processor.channel_data.len()
                        {
                            warn_once!(
                                "Using AudioControlConfig with channel \
                                beyond available device channels: {:?}",
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

                        Some((name.clone(), mapped, config.channel, value))
                    })
                    .collect();

                for (name, mapped, channel, value) in updates {
                    state.values.insert(name, mapped);
                    state.previous_values[channel] = value;
                }
            },
            move |err| error!("Error in audio stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        self.is_active = true;

        info!(
            "AudioControls connected to device: {:?}",
            device.name().unwrap()
        );

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(_stream) = self.stream.take() {
            self.is_active = false;
            debug!("Audio stream stopped");
        }
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.stop();
        thread::sleep(Duration::from_millis(10));
        self.start()
    }

    fn device_and_stream_config(
    ) -> Result<(Device, StreamConfig), Box<dyn Error>> {
        let host = cpal::default_host();
        let device_name = global::audio_device_name();
        let device = host
            .input_devices()?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .expect("Audio device not found");

        let stream_config = device.default_input_config()?.into();

        Ok((device, stream_config))
    }
}

pub struct AudioControlBuilder {
    controls: AudioControls,
}

impl Default for AudioControlBuilder {
    fn default() -> Self {
        Self {
            controls: AudioControls::new(default_buffer_processor),
        }
    }
}

impl AudioControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// See [`BufferProcessor`]
    pub fn with_buffer_processor(
        mut self,
        buffer_processor: BufferProcessor,
    ) -> Self {
        self.controls.buffer_processor = buffer_processor;
        self
    }

    pub fn control_from_config(
        mut self,
        name: &str,
        config: AudioControlConfig,
    ) -> Self {
        self.controls.add(name, config);
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
                "Failed to initialize audio controls: {}. \
                Using default values.",
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
                let ch_data = self.channel_data.get_mut(channel);
                if let Some(buffer) = ch_data {
                    buffer.push(sample);
                }
            }
        }

        // Ensure buffers are filled to their exact size
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

    /// Standard pre-emphasis filter `y[n] = x[n] - α * x[n-1]` that amplifies
    /// high frequencies relative to low frequencies by subtracting a portion of
    /// the previous sample. It boosts high frequencies indirectly rather than
    /// explicitly cutting low frequencies with a sharp cutoff like a classical
    /// HPF. 0.97 is common as it gives about +20dB emphasis starting around
    /// 1kHz. See freq-response-pre-emphasis.png in the repo's assets folder for
    /// more details.
    pub fn apply_pre_emphasis(buffer: &[f32], coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(buffer.len());
        filtered.push(buffer[0]);

        for i in 1..buffer.len() {
            filtered.push(buffer[i] - coefficient * buffer[i - 1]);
        }

        filtered
    }

    /// Linearly interpolate between peak and RMS amplitude detection. 0.0 =
    /// peak, 1.0 = RMS
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
