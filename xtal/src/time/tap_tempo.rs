use std::collections::VecDeque;
use std::time::{Duration, Instant};

const MAX_TAPS: usize = 4;
const MIN_BPM: f32 = 30.0;
const MAX_BPM: f32 = 300.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TapTempoMode {
    LegacyTwoTap,
    WindowedPhaseLock,
}

impl TapTempoMode {
    pub fn from_env() -> Self {
        match std::env::var("XTAL_TAP_TEMPO_MODE") {
            Ok(value) if value == "legacy" => Self::LegacyTwoTap,
            _ => Self::WindowedPhaseLock,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapTempoResult {
    pub bpm: f32,
    pub phase_lock: bool,
}

pub struct TapTempo {
    bpm: f32,
    previous_timestamp: Instant,
    timeout: Duration,
    mode: TapTempoMode,
    taps: VecDeque<Instant>,
}

impl TapTempo {
    pub fn new(bpm: f32) -> Self {
        Self::new_with_mode(bpm, TapTempoMode::from_env())
    }

    pub fn new_with_mode(bpm: f32, mode: TapTempoMode) -> Self {
        Self {
            bpm,
            previous_timestamp: Instant::now(),
            timeout: Duration::from_secs(2),
            mode,
            taps: VecDeque::with_capacity(MAX_TAPS),
        }
    }

    pub fn tap(&mut self) -> TapTempoResult {
        let now = Instant::now();
        self.tap_at(now)
    }

    fn tap_at(&mut self, now: Instant) -> TapTempoResult {
        match self.mode {
            TapTempoMode::LegacyTwoTap => self.legacy_tap_at(now),
            TapTempoMode::WindowedPhaseLock => self.windowed_tap_at(now),
        }
    }

    fn legacy_tap_at(&mut self, now: Instant) -> TapTempoResult {
        let difference = now.duration_since(self.previous_timestamp);

        if difference <= self.timeout {
            self.bpm = 60.0 / difference.as_secs_f32();
        }

        self.previous_timestamp = now;
        TapTempoResult {
            bpm: self.bpm,
            phase_lock: false,
        }
    }

    fn windowed_tap_at(&mut self, now: Instant) -> TapTempoResult {
        if let Some(previous) = self.taps.back() {
            if now.duration_since(*previous) > self.timeout {
                self.taps.clear();
            }
        }

        self.taps.push_back(now);
        while self.taps.len() > MAX_TAPS {
            self.taps.pop_front();
        }

        if let Some(bpm) = self.estimate_windowed_bpm() {
            self.bpm = bpm;
        }

        self.previous_timestamp = now;
        TapTempoResult {
            bpm: self.bpm,
            phase_lock: true,
        }
    }

    fn estimate_windowed_bpm(&self) -> Option<f32> {
        if self.taps.len() < 2 {
            return None;
        }

        let bpm = if self.taps.len() < 4 {
            self.estimate_span_bpm()?
        } else {
            self.estimate_linear_fit_bpm()?
        };

        Some(bpm.clamp(MIN_BPM, MAX_BPM))
    }

    fn estimate_span_bpm(&self) -> Option<f32> {
        let first = *self.taps.front()?;
        let last = *self.taps.back()?;
        // Early taps use the whole sequence span so small tap errors can
        // cancel out before there are enough points for the linear fit.
        let beats = self.taps.len().saturating_sub(1) as f32;
        let seconds = last.duration_since(first).as_secs_f32();

        if seconds <= 0.0 {
            return None;
        }

        Some(60.0 * beats / seconds)
    }

    fn estimate_linear_fit_bpm(&self) -> Option<f32> {
        let first = *self.taps.front()?;
        let count = self.taps.len() as f32;
        let mut sum_index = 0.0;
        let mut sum_time = 0.0;
        let mut sum_index_time = 0.0;
        let mut sum_index_squared = 0.0;

        for (i, tap) in self.taps.iter().enumerate() {
            let index = i as f32;
            let seconds = tap.duration_since(first).as_secs_f32();
            sum_index += index;
            sum_time += seconds;
            sum_index_time += index * seconds;
            sum_index_squared += index * index;
        }

        let denominator = count * sum_index_squared - sum_index * sum_index;
        if denominator <= 0.0 {
            return None;
        }

        let seconds_per_beat =
            (count * sum_index_time - sum_index * sum_time) / denominator;
        if seconds_per_beat <= 0.0 {
            return None;
        }

        Some(60.0 / seconds_per_beat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant_after(base: Instant, seconds: f32) -> Instant {
        base + Duration::from_secs_f32(seconds)
    }

    #[test]
    fn legacy_mode_uses_only_latest_interval() {
        let mut tap_tempo =
            TapTempo::new_with_mode(120.0, TapTempoMode::LegacyTwoTap);
        let start = Instant::now();

        tap_tempo.tap_at(start);
        let result = tap_tempo.tap_at(instant_after(start, 0.4));

        assert!((result.bpm - 150.0).abs() < 0.001);
        assert!(!result.phase_lock);
    }

    #[test]
    fn windowed_mode_uses_full_span_for_early_taps() {
        let mut tap_tempo =
            TapTempo::new_with_mode(120.0, TapTempoMode::WindowedPhaseLock);
        let start = Instant::now();

        tap_tempo.tap_at(start);
        tap_tempo.tap_at(instant_after(start, 0.5));
        let result = tap_tempo.tap_at(instant_after(start, 1.0));

        assert!((result.bpm - 120.0).abs() < 0.001);
        assert!(result.phase_lock);
    }

    #[test]
    fn windowed_mode_smooths_one_late_tap() {
        let mut tap_tempo =
            TapTempo::new_with_mode(120.0, TapTempoMode::WindowedPhaseLock);
        let start = Instant::now();

        tap_tempo.tap_at(start);
        tap_tempo.tap_at(instant_after(start, 0.5));
        tap_tempo.tap_at(instant_after(start, 1.0));
        tap_tempo.tap_at(instant_after(start, 1.5));
        let result = tap_tempo.tap_at(instant_after(start, 2.08));

        assert!(result.bpm > 112.0);
        assert!(result.bpm < 120.0);
        assert!(result.phase_lock);
    }

    #[test]
    fn windowed_mode_resets_after_timeout() {
        let mut tap_tempo =
            TapTempo::new_with_mode(120.0, TapTempoMode::WindowedPhaseLock);
        let start = Instant::now();

        tap_tempo.tap_at(start);
        tap_tempo.tap_at(instant_after(start, 0.5));
        tap_tempo.tap_at(instant_after(start, 3.0));
        let result = tap_tempo.tap_at(instant_after(start, 3.75));

        assert!((result.bpm - 80.0).abs() < 0.001);
        assert!(result.phase_lock);
    }
}
