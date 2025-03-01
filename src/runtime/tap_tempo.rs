use std::time::Instant;

#[allow(unused_imports)]
use crate::framework::prelude::*;

pub struct TapTempo {
    previous_timestamp: Instant,
    bpm: f32,
}

impl TapTempo {
    pub fn new() -> Self {
        Self {
            previous_timestamp: Instant::now(),
            bpm: 120.0,
        }
    }

    pub fn tap(&mut self) -> f32 {
        self.tap_with_instant(Instant::now())
    }

    pub fn tap_with_instant(&mut self, now: Instant) -> f32 {
        let difference = now.duration_since(self.previous_timestamp);
        self.bpm = 60.0 / difference.as_secs_f32();
        self.previous_timestamp = now;
        self.bpm
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }
}

#[cfg(test)]
mod tests {
    use crate::framework::util::tests::approx_eq;
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_tap_tempo() {
        let mut tap_tempo = TapTempo::new();

        let t1 = Instant::now();
        tap_tempo.tap_with_instant(t1);

        let t2 = t1 + Duration::from_secs_f64(447.78 / 1000.0);
        tap_tempo.tap_with_instant(t2);

        approx_eq(tap_tempo.bpm(), 134.0);
        // assert_eq!(tap_tempo.bpm(), 134.0);
    }
}
