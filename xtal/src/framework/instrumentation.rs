//! Bare-bones instrumentation for quick insights

#[cfg(feature = "instrumentation")]
use std::time::{Duration, Instant};

#[cfg(feature = "instrumentation")]
use super::prelude::*;

#[cfg(feature = "instrumentation")]
#[derive(Debug)]
pub struct Instrumentation {
    label: String,
    total_duration: Duration,
    call_count: usize,
    last_report_time: Instant,
    report_interval: Duration,
}

#[cfg(feature = "instrumentation")]
impl Instrumentation {
    pub fn new(label: &str) -> Self {
        info!("Instrumentation enabled");
        Self {
            label: label.to_string(),
            total_duration: Duration::ZERO,
            call_count: 0,
            last_report_time: Instant::now(),
            report_interval: Duration::from_secs(1),
        }
    }

    pub fn start(&self) -> Instant {
        Instant::now()
    }

    pub fn record(&mut self, start_time: Instant) {
        let elapsed = start_time.elapsed();
        self.total_duration += elapsed;
        self.call_count += 1;

        let now = Instant::now();
        if now.duration_since(self.last_report_time) >= self.report_interval {
            self.report();
            self.total_duration = Duration::ZERO;
            self.call_count = 0;
            self.last_report_time = now;
        }
    }

    pub fn report(&self) {
        if self.call_count > 0 {
            let avg_duration = self.total_duration / self.call_count as u32;
            info!("[{}] Average: {:.2?}", self.label, avg_duration);
        }
    }
}
