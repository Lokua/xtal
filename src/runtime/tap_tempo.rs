use std::time::{Duration, Instant};

#[allow(unused_imports)]
use crate::framework::prelude::*;

pub struct TapTempo {
    bpm: f32,
    previous_timestamp: Instant,
    timeout: Duration,
}

impl TapTempo {
    pub fn new(bpm: f32) -> Self {
        Self {
            bpm,
            previous_timestamp: Instant::now(),
            timeout: Duration::from_secs(2),
        }
    }

    pub fn tap(&mut self) -> f32 {
        self.tap_with_instant(Instant::now())
    }

    pub fn tap_with_instant(&mut self, now: Instant) -> f32 {
        let difference = now.duration_since(self.previous_timestamp);

        if difference <= self.timeout {
            self.bpm = 60.0 / difference.as_secs_f32();
        }

        self.previous_timestamp = now;

        self.bpm
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_approx_eq;
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_tap_tempo() {
        let mut tap_tempo = TapTempo::new(120.0);

        let t1 = Instant::now();
        tap_tempo.tap_with_instant(t1);

        let t2 = t1 + Duration::from_secs_f64(447.78 / 1000.0);
        tap_tempo.tap_with_instant(t2);

        assert_approx_eq!(tap_tempo.bpm(), 134.0, 0.01);
    }

    #[test]
    fn test_tap_tempo_with_timeout() {
        let mut tap_tempo = TapTempo::new(120.0);

        let t1 = Instant::now();
        tap_tempo.tap_with_instant(t1);

        let t2 = t1 + Duration::from_secs_f64(447.78 / 1000.0);
        tap_tempo.tap_with_instant(t2);

        assert_approx_eq!(tap_tempo.bpm(), 134.0, 0.01);

        let t3 = t2 + Duration::from_secs(5);
        tap_tempo.tap_with_instant(t3);

        // BPM should remain unchanged after timeout
        assert_approx_eq!(tap_tempo.bpm(), 134.0, 0.01);
    }
}
