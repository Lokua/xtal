//! Experimental helpers for deriving sketch values from live audio input.

use cpal::{Device, Stream, StreamConfig, traits::*};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::cmp::Ordering;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::core::prelude::*;
use crate::time::frame_clock;

#[derive(Debug, Clone, Copy)]
struct SlewConfig {
    pub rise: f32,
    pub fall: f32,
}

impl Default for SlewConfig {
    fn default() -> Self {
        Self {
            rise: 0.15,
            fall: 0.5,
        }
    }
}

#[derive(Default)]
pub struct Audio {
    audio_processor: Arc<Mutex<AudioProcessor>>,
    slew_config: SlewConfig,
    previous_band_values: Vec<f32>,
    cutoffs: Vec<f32>,
    device_name: Option<String>,
    stream: Option<Stream>,
    is_active: bool,
}

impl Audio {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_device_name(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.device_name = if name.is_empty() { None } else { Some(name) };
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(device_name) = self.device_name.clone() else {
            warn!("Skipping Audio setup; no audio device selected.");
            return Ok(());
        };

        let (device, stream_config) =
            Self::device_and_stream_config(&device_name)?;

        {
            let mut processor = self.audio_processor.lock().unwrap();
            processor.initialize(stream_config.sample_rate.0 as usize);
        }

        let shared_audio = self.audio_processor.clone();
        let channels = stream_config.channels;

        if channels < 1 {
            return Err("Device must have at least one channel".into());
        }

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                let left_channel: Vec<f32> =
                    data.iter().step_by(channels as usize).copied().collect();
                let mut processor = shared_audio.lock().unwrap();
                processor.add_samples(&left_channel);
            },
            move |err| error!("Error in audio stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        self.is_active = true;
        info!(
            "Audio connected to device: {:?}",
            device.name().unwrap_or_else(|_| "Unknown".to_string())
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
        std::thread::sleep(std::time::Duration::from_millis(10));
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

    #[allow(clippy::too_many_arguments)]
    pub fn bands(
        &mut self,
        n_bands: usize,
        min_freq: f32,
        max_freq: f32,
        pre_emphasis: f32,
        rise: f32,
        fall: f32,
    ) -> Vec<f32> {
        let audio_processor = self.audio_processor.lock().unwrap();
        let emphasized = audio_processor.apply_pre_emphasis(pre_emphasis);

        if self.cutoffs.is_empty() {
            self.cutoffs = audio_processor.generate_mel_cutoffs(
                n_bands + 1,
                min_freq,
                max_freq,
            );
        }

        let bands =
            audio_processor.bands_from_buffer(&emphasized, &self.cutoffs);

        self.slew_config.rise = rise;
        self.slew_config.fall = fall;

        if self.previous_band_values.is_empty() {
            self.previous_band_values = vec![0.0; n_bands];
        }

        let smoothed = audio_processor.follow_band_envelopes(
            bands,
            self.slew_config,
            &self.previous_band_values,
        );
        self.previous_band_values = smoothed.clone();

        smoothed
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

struct AudioProcessor {
    sample_rate: usize,
    buffer: Vec<f32>,
    buffer_size: usize,
    fft: Option<Arc<dyn Fft<f32>>>,
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            buffer: Vec::new(),
            buffer_size: 1024,
            fft: None,
        }
    }
}

impl AudioProcessor {
    pub fn initialize(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        let frame_rate = frame_clock::fps();
        self.buffer_size = (sample_rate as f32 / frame_rate).ceil() as usize;
        self.buffer = vec![0.0; self.buffer_size];
        let mut planner = FftPlanner::new();
        self.fft = Some(planner.plan_fft_forward(self.buffer_size));
    }

    pub fn add_samples(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);

        match self.buffer.len().cmp(&self.buffer_size) {
            Ordering::Greater => {
                self.buffer.drain(0..(self.buffer.len() - self.buffer_size));
            }
            Ordering::Less => {
                while self.buffer.len() < self.buffer_size {
                    self.buffer.push(0.0);
                }
            }
            Ordering::Equal => {}
        }
    }

    pub fn apply_pre_emphasis(&self, coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(self.buffer.len());
        filtered.push(self.buffer[0]);

        for i in 1..self.buffer.len() {
            filtered.push(self.buffer[i] - coefficient * self.buffer[i - 1]);
        }

        filtered
    }

    pub fn bands_from_buffer(
        &self,
        buffer: &[f32],
        cutoffs: &[f32],
    ) -> Vec<f32> {
        let fft = match &self.fft {
            Some(fft) => fft,
            None => return vec![0.0; cutoffs.len().saturating_sub(1)],
        };

        let mut complex_input: Vec<Complex<f32>> =
            buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();
        fft.process(&mut complex_input);

        let freq_resolution = (self.sample_rate / complex_input.len()) as f32;
        let stops: Vec<usize> = cutoffs
            .iter()
            .map(|cutoff| (cutoff / freq_resolution).round() as usize)
            .collect();

        let magnitudes: Vec<f32> = complex_input
            .iter()
            .map(|c| {
                let magnitude = c.norm() / complex_input.len() as f32;
                20.0 * (magnitude.max(1e-8)).log10()
            })
            .collect();

        let get_band_magnitude = |start: usize, end: usize| -> f32 {
            let slice = &magnitudes[start..end.min(magnitudes.len())];
            if slice.is_empty() {
                return -80.0;
            }
            *slice
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(&-80.0)
        };

        let normalize = |db: f32| ((db + 80.0) / 60.0).clamp(0.0, 1.0);

        stops
            .iter()
            .take(stops.len().saturating_sub(1))
            .enumerate()
            .map(|(index, &stop)| get_band_magnitude(stop, stops[index + 1]))
            .map(normalize)
            .collect()
    }

    pub fn follow_envelope(
        &self,
        input: &[f32],
        config: SlewConfig,
        previous: f32,
    ) -> Vec<f32> {
        let mut envelope = Vec::with_capacity(input.len());
        let mut current = previous;
        for &sample in input {
            let magnitude = sample.abs();
            let coeff = if magnitude > current {
                1.0 - config.rise
            } else {
                1.0 - config.fall
            };
            current = current + coeff * (magnitude - current);
            envelope.push(current);
        }
        envelope
    }

    pub fn follow_band_envelopes(
        &self,
        bands: Vec<f32>,
        config: SlewConfig,
        previous_values: &[f32],
    ) -> Vec<f32> {
        bands
            .iter()
            .enumerate()
            .map(|(i, &band)| {
                let prev = previous_values.get(i).copied().unwrap_or(0.0);
                self.follow_envelope(&[band], config, prev)[0]
            })
            .collect()
    }

    fn hz_to_mel(freq: f32) -> f32 {
        2595.0 * (1.0 + freq / 700.0).log10()
    }

    fn mel_to_hz(mel: f32) -> f32 {
        700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
    }

    pub fn generate_mel_cutoffs(
        &self,
        num_bands: usize,
        min_freq: f32,
        max_freq: f32,
    ) -> Vec<f32> {
        assert!(num_bands >= 2, "Number of bands must be at least 2");
        assert!(min_freq < max_freq, "min_freq must be less than max_freq");

        let min_mel = Self::hz_to_mel(min_freq);
        let max_mel = Self::hz_to_mel(max_freq);
        let mel_step = (max_mel - min_mel) / num_bands as f32;

        let mut cutoffs = Vec::with_capacity(num_bands + 1);
        for i in 0..=num_bands {
            let mel = min_mel + mel_step * i as f32;
            cutoffs.push(Self::mel_to_hz(mel));
        }
        cutoffs
    }
}

pub fn list_audio_devices() -> Result<Vec<String>, Box<dyn Error>> {
    let audio_host = cpal::default_host();
    let devices = audio_host.input_devices()?;
    let info = devices
        .map(|device| {
            let name = device.name()?;
            Ok::<String, Box<dyn Error>>(name)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(info)
}
