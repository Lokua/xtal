use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TickResult {
    pub should_render: bool,
    pub frames_advanced: u32,
}

#[derive(Debug)]
pub struct FrameClock {
    fps: f32,
    frame_count: u64,
    paused: bool,
    force_render: bool,
    last_tick: Instant,
    accumulator: Duration,
    frame_intervals: VecDeque<Duration>,
    max_intervals: usize,
}

impl FrameClock {
    pub fn new(fps: f32) -> Self {
        let now = Instant::now();
        Self::with_start(fps, now)
    }

    pub fn with_start(fps: f32, now: Instant) -> Self {
        Self {
            fps: fps.max(1.0),
            frame_count: 0,
            paused: false,
            force_render: false,
            last_tick: now,
            accumulator: Duration::ZERO,
            frame_intervals: VecDeque::new(),
            max_intervals: 90,
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.max(1.0);
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn advance_single_frame(&mut self) {
        self.force_render = true;
    }

    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.fps)
    }

    pub fn next_deadline(&self) -> Instant {
        let remaining = self
            .frame_duration()
            .checked_sub(self.accumulator)
            .unwrap_or_default();
        self.last_tick + remaining
    }

    pub fn average_fps(&self) -> f32 {
        if self.frame_intervals.is_empty() {
            return 0.0;
        }

        let sum: Duration = self.frame_intervals.iter().copied().sum();
        let avg = sum / self.frame_intervals.len() as u32;

        if avg.is_zero() {
            return 0.0;
        }

        1.0 / avg.as_secs_f32()
    }

    pub fn tick(&mut self, now: Instant) -> TickResult {
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        self.accumulator += elapsed;

        if self.force_render {
            self.force_render = false;
            self.frame_count += 1;
            self.record_interval(elapsed);
            return TickResult {
                should_render: true,
                frames_advanced: 1,
            };
        }

        if self.paused {
            // While paused we do not accumulate debt; resume should continue
            // from "now" rather than catching up missed frames.
            self.accumulator = Duration::ZERO;
            return TickResult::default();
        }

        let frame_duration = self.frame_duration();
        let mut advanced = 0u32;

        while self.accumulator >= frame_duration {
            self.accumulator -= frame_duration;
            self.frame_count += 1;
            advanced += 1;
        }

        if advanced > 0 {
            self.record_interval(elapsed);
            TickResult {
                should_render: true,
                frames_advanced: advanced,
            }
        } else {
            TickResult::default()
        }
    }

    fn record_interval(&mut self, interval: Duration) {
        self.frame_intervals.push_back(interval);
        if self.frame_intervals.len() > self.max_intervals {
            self.frame_intervals.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn frame_clock_advances_on_full_interval() {
        let start = Instant::now();
        let mut clock = FrameClock::with_start(60.0, start);

        let half = start + clock.frame_duration() / 2;
        assert_eq!(clock.tick(half), TickResult::default());

        let full = half + clock.frame_duration() / 2;
        let tick = clock.tick(full);
        assert!(tick.should_render);
        assert_eq!(tick.frames_advanced, 1);
        assert_eq!(clock.frame_count(), 1);
    }

    #[test]
    fn frame_clock_catches_up_when_lagging() {
        let start = Instant::now();
        let mut clock = FrameClock::with_start(30.0, start);
        let now = start + clock.frame_duration() * 3;

        let tick = clock.tick(now);
        assert!(tick.should_render);
        assert_eq!(tick.frames_advanced, 3);
        assert_eq!(clock.frame_count(), 3);
    }

    #[test]
    fn frame_clock_pause_and_advance() {
        let start = Instant::now();
        let mut clock = FrameClock::with_start(60.0, start);
        clock.set_paused(true);

        let now = start + Duration::from_secs(1);
        assert_eq!(clock.tick(now), TickResult::default());
        assert_eq!(clock.frame_count(), 0);

        clock.advance_single_frame();
        let tick = clock.tick(now + Duration::from_millis(1));
        assert!(tick.should_render);
        assert_eq!(tick.frames_advanced, 1);
        assert_eq!(clock.frame_count(), 1);
    }

    #[test]
    fn frame_clock_applies_runtime_fps_changes() {
        let start = Instant::now();
        let mut clock = FrameClock::with_start(60.0, start);

        let at_60hz = start + clock.frame_duration();
        let tick = clock.tick(at_60hz);
        assert!(tick.should_render);
        assert_eq!(tick.frames_advanced, 1);

        clock.set_fps(30.0);
        assert_eq!(clock.fps(), 30.0);

        // Partial 30fps frame should not render.
        let partial_30hz = at_60hz + clock.frame_duration() / 3;
        assert_eq!(clock.tick(partial_30hz), TickResult::default());

        let full_30hz = at_60hz + clock.frame_duration();
        let tick = clock.tick(full_30hz);
        assert!(tick.should_render);
        assert_eq!(tick.frames_advanced, 1);
        assert_eq!(clock.frame_count(), 2);
    }
}
