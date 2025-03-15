//! **⚠️ Experimental**
//! Receive single-channel, multiband audio with configurable FFT bands.

use cpal::traits::*;
use cpal::BuildStreamError;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use super::prelude::*;

/// Configuration for envelope following behavior, controlling how quickly the
/// envelope tracks changes in the input signal.
/// TODO: deprecate and move to SlewLimiter
#[derive(Debug, Clone, Copy)]
pub struct SlewConfig {
    /// Controls smoothing when signal amplitude increases.
    /// - 0.0 = instant attack (no smoothing)
    /// - 1.0 = very slow attack (maximum smoothing)
    pub rise: f32,

    /// Controls smoothing when signal amplitude decreases.
    /// - 0.0 = instant decay (no smoothing)
    /// - 1.0 = very slow decay (maximum smoothing)
    pub fall: f32,
}

impl SlewConfig {
    pub fn new(rise: f32, fall: f32) -> Self {
        Self { rise, fall }
    }
}

impl Default for SlewConfig {
    fn default() -> Self {
        Self {
            rise: 0.15,
            fall: 0.5,
        }
    }
}

/// **⚠️ Experimental**
/// Single-channel, multiband audio with configurable FFT bands.
pub struct Audio {
    audio_processor: Arc<Mutex<AudioProcessor>>,
    slew_config: SlewConfig,
    previous_band_values: Vec<f32>,
    cutoffs: Vec<f32>,
}

impl Audio {
    pub fn new(sample_rate: usize, frame_rate: f32) -> Self {
        let audio_processor =
            Arc::new(Mutex::new(AudioProcessor::new(sample_rate, frame_rate)));

        init_audio(audio_processor.clone()).expect("Unable to init audio");

        Self {
            audio_processor,
            slew_config: SlewConfig::default(),
            previous_band_values: vec![],
            cutoffs: vec![],
        }
    }

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
            )
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
}

struct AudioProcessor {
    sample_rate: usize,
    buffer: Vec<f32>,
    buffer_size: usize,
    fft: Arc<dyn Fft<f32>>,
}

impl AudioProcessor {
    pub fn new(sample_rate: usize, frame_rate: f32) -> Self {
        let buffer_size = (sample_rate as f32 / frame_rate).ceil() as usize;
        trace!("buffer_size: {}", buffer_size);
        let mut planner: FftPlanner<f32> = FftPlanner::new();
        let fft = planner.plan_fft_forward(buffer_size);

        Self {
            sample_rate,
            buffer: vec![0.0; buffer_size],
            buffer_size,
            fft,
        }
    }

    pub fn add_samples(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);

        // Deal with possible race condition of sketch update happening and
        // requesting data before the buffer is full.
        // "Provided FFT buffer was too small. Expected len = 1600, got len = 1536"
        match self.buffer.len().cmp(&self.buffer_size) {
            Ordering::Greater => {
                self.buffer.drain(0..(self.buffer.len() - self.buffer_size));
            }
            Ordering::Less => {
                while self.buffer.len() < self.buffer_size {
                    self.buffer.push(0.0);
                }
            }
            _ => {}
        };
    }

    /// Standard pre-emphasis filter: `y[n] = x[n] - α * x[n-1]`
    /// 0.97 is common is it gives about +20dB emphasis starting around 1kHz
    pub fn apply_pre_emphasis(&self, coefficient: f32) -> Vec<f32> {
        let mut filtered = Vec::with_capacity(self.buffer.len());
        filtered.push(self.buffer[0]);

        for i in 1..self.buffer.len() {
            filtered.push(self.buffer[i] - coefficient * self.buffer[i - 1]);
        }

        filtered
    }

    #[allow(dead_code)]
    pub fn peak(&self) -> f32 {
        self.buffer.iter().fold(f32::MIN, |a, &b| f32::max(a, b))
    }

    #[allow(dead_code)]
    pub fn rms(&self) -> f32 {
        (self.buffer.iter().map(|&x| x * x).sum::<f32>()
            / self.buffer.len() as f32)
            .sqrt()
    }

    #[allow(dead_code)]
    pub fn bands(&self, cutoffs: &[f32]) -> Vec<f32> {
        self.bands_from_buffer(&self.buffer, cutoffs)
    }

    pub fn bands_from_buffer(
        &self,
        buffer: &[f32],
        cutoffs: &[f32],
    ) -> Vec<f32> {
        let mut complex_input: Vec<Complex<f32>> =
            buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();

        self.fft.process(&mut complex_input);

        let freq_resolution = (self.sample_rate / complex_input.len()) as f32;

        // Convert frequency cutoffs to bin indices, maintaining precision until indexing
        let stops: Vec<usize> = cutoffs
            .iter()
            .map(|cutoff| (cutoff / freq_resolution).round() as usize)
            .collect();

        trace!("freq_resolution: {}, stops {:?}", freq_resolution, stops);

        // Calculate magnitude and convert to dB for each bin
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
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        };

        let normalize = |db: f32| ((db + 80.0) / 60.0).clamp(0.0, 1.0);

        let bands: Vec<f32> = stops
            .iter()
            .take(stops.len() - 1)
            .enumerate()
            .map(|(index, &stop)| get_band_magnitude(stop, stops[index + 1]))
            .map(normalize)
            .collect();

        bands
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

    /// Generates logarithmically spaced frequency cutoffs in Hz for the
    /// specified number of bands. Lower bands have custom spacing to avoid
    /// multiple bands mapping to the same fft bin index which results in
    /// empties. Works OK for <= 32 bands but starts to produce gaps at
    /// higher band counts.
    ///
    /// # Arguments
    /// * `num_bands` - Number of bands to generate (4, 8, 16, 32, 128, or 256)
    /// * `min_freq` - Minimum frequency in Hz (typically 20-100 Hz)
    /// * `max_freq` - Maximum frequency in Hz (typically 10000-20000 Hz)
    ///
    /// # Returns
    /// Vector of frequency cutoffs in Hz as usize values
    #[allow(dead_code)]
    pub fn generate_cutoffs(
        &self,
        num_bands: usize,
        min_freq: f32,
        max_freq: f32,
    ) -> Vec<f32> {
        assert!(num_bands >= 2, "Number of bands must be at least 2");
        assert!(min_freq < max_freq, "min_freq must be less than max_freq");

        let mut cutoffs = Vec::with_capacity(num_bands + 1);
        cutoffs.push(min_freq);

        // Calculate our actual frequency resolution
        let freq_resolution = self.sample_rate as f32 / self.buffer_size as f32;

        // For the first few bands, ensure minimum width of 1.5 * freq_resolution
        // This helps ensure each band maps to at least one unique FFT bin
        let min_band_width = freq_resolution * 1.5;

        // Handle first few bands with fixed minimum widths
        let mut current_freq = min_freq;

        // Frequency at which we switch to logarithmic spacing
        let transition_freq = 300.0;

        while current_freq < transition_freq && cutoffs.len() < num_bands {
            current_freq += min_band_width;
            cutoffs.push(current_freq);
        }

        // If we still need more bands, continue with logarithmic spacing
        if cutoffs.len() < num_bands + 1 {
            let remaining_bands = num_bands + 1 - cutoffs.len();
            let factor =
                (max_freq / current_freq).powf(1.0 / remaining_bands as f32);

            for _ in 0..remaining_bands {
                current_freq *= factor;
                cutoffs.push(current_freq);
            }
        }

        // Ensure the last cutoff is exactly max_freq
        if let Some(last) = cutoffs.last_mut() {
            *last = max_freq;
        }

        cutoffs
    }

    /// Convert frequency in Hz to Mel scale
    fn hz_to_mel(freq: f32) -> f32 {
        2595.0 * (1.0 + freq / 700.0).log10()
    }

    /// Convert Mel scale back to frequency in Hz
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

        let mut cutoffs = Vec::with_capacity(num_bands + 1);

        // Convert frequency range to Mel scale
        let min_mel = Self::hz_to_mel(min_freq);
        let max_mel = Self::hz_to_mel(max_freq);

        // Create linearly spaced points in Mel scale
        let mel_step = (max_mel - min_mel) / num_bands as f32;

        // Convert back to Hz
        for i in 0..=num_bands {
            let mel = min_mel + mel_step * i as f32;
            let hz = Self::mel_to_hz(mel);
            cutoffs.push(hz);
        }

        cutoffs
    }
}

fn init_audio(
    shared_audio: Arc<Mutex<AudioProcessor>>,
) -> Result<(), BuildStreamError> {
    let audio_host = cpal::default_host();
    let devices: Vec<_> = audio_host.input_devices().unwrap().collect();
    let audio_device_name = crate::config::AUDIO_DEVICE_NAME;

    let device = devices
        .into_iter()
        .find(|device| {
            let name = device.name().unwrap();
            trace!("Enumerating devices. Device name: {}", name);
            let found = device.name().unwrap() == audio_device_name;
            if found {
                trace!("Using: {}", audio_device_name);
            }
            found
        })
        .unwrap_or_else(|| {
            panic!("No device named {} found", audio_device_name)
        });

    let input_config = match device.default_input_config() {
        Ok(config) => {
            trace!("Default output stream config: {:?}", config);
            config
        }
        Err(err) => {
            panic!("Failed to get default output config: {:?}", err);
        }
    };

    let stream = match input_config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &input_config.into(),
            move |source_data: &[f32], _| {
                // There is no concept of "channels" for a device.
                // All channel data is stored in a flat array:
                //
                //      let channel_1 = source_data.iter().step_by(16).cloned();
                //      let channel_8 = source_data.iter().skip(7).step_by(16).cloned();
                //      let channel_16 = source_data.iter().skip(15).step_by(16).cloned();
                //
                // Left = even, Right = odd;
                // Do `data.iter().skip(1).step_by(2)` for right
                let data: Vec<f32> =
                    source_data.iter().step_by(2).cloned().collect();

                let mut audio_processor = shared_audio.lock().unwrap();

                audio_processor.add_samples(&data);
            },
            move |err| error!("An error occurred on steam: {}", err),
            None,
        )?,
        sample_format => {
            panic!("Unsupported sample format {}", sample_format);
        }
    };

    let _ = stream.play();

    Ok(())
}
