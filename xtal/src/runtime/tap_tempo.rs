use std::time::{Duration, Instant};

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
        let now = Instant::now();
        let difference = now.duration_since(self.previous_timestamp);

        if difference <= self.timeout {
            self.bpm = 60.0 / difference.as_secs_f32();
        }

        self.previous_timestamp = now;
        self.bpm
    }
}
