use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::framework::util::AtomicF32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TickResult {
    pub should_render: bool,
    pub frames_advanced: u32,
}

struct Pacer {
    last_tick: Instant,
    accumulator: Duration,
    frame_intervals: VecDeque<Duration>,
    last_render_at: Option<Instant>,
    max_intervals: usize,
    force_render: bool,
}

impl Pacer {
    fn new(now: Instant) -> Self {
        Self {
            last_tick: now,
            accumulator: Duration::ZERO,
            frame_intervals: VecDeque::new(),
            last_render_at: None,
            max_intervals: 90,
            force_render: false,
        }
    }

    fn reset_timing(&mut self, now: Instant) {
        self.last_tick = now;
        self.accumulator = Duration::ZERO;
        self.frame_intervals.clear();
        self.last_render_at = None;
        self.force_render = false;
    }

    fn tick(&mut self, now: Instant) -> TickResult {
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        self.accumulator += elapsed;

        if self.force_render {
            self.force_render = false;
            advance_frames(1);
            self.record_render(now);
            return TickResult {
                should_render: true,
                frames_advanced: 1,
            };
        }

        if paused() {
            // While paused we do not accumulate debt.
            self.accumulator = Duration::ZERO;
            return TickResult::default();
        }

        let frame_duration = frame_duration();
        let mut advanced = 0u32;

        while self.accumulator >= frame_duration {
            self.accumulator -= frame_duration;
            advanced += 1;
        }

        if advanced > 0 {
            advance_frames(advanced);
            self.record_render(now);
            TickResult {
                should_render: true,
                frames_advanced: advanced,
            }
        } else {
            TickResult::default()
        }
    }

    fn next_deadline(&self) -> Instant {
        let remaining = frame_duration()
            .checked_sub(self.accumulator)
            .unwrap_or_default();
        self.last_tick + remaining
    }

    fn average_fps(&self) -> f32 {
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

    fn record_render(&mut self, now: Instant) {
        let Some(last_render_at) = self.last_render_at else {
            self.last_render_at = Some(now);
            return;
        };

        let interval = now.saturating_duration_since(last_render_at);
        self.last_render_at = Some(now);
        self.frame_intervals.push_back(interval);
        if self.frame_intervals.len() > self.max_intervals {
            self.frame_intervals.pop_front();
        }
    }
}

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static FPS: AtomicF32 = AtomicF32::new(60.0);
static PAUSED: AtomicBool = AtomicBool::new(false);
static PACER: LazyLock<Mutex<Pacer>> =
    LazyLock::new(|| Mutex::new(Pacer::new(Instant::now())));

fn with_pacer<R>(f: impl FnOnce(&mut Pacer) -> R) -> R {
    let mut pacer = PACER.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut pacer)
}

pub fn frame_count() -> u32 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

pub fn set_frame_count(count: u32) {
    FRAME_COUNT.store(count, Ordering::Relaxed);
}

pub fn advance_frames(count: u32) {
    if count > 0 {
        FRAME_COUNT.fetch_add(count, Ordering::Relaxed);
    }
}

pub fn reset_frame_count() {
    set_frame_count(0);
}

pub fn reset() {
    reset_frame_count();
    reset_timing(Instant::now());
}

pub fn fps() -> f32 {
    FPS.load(Ordering::Acquire)
}

pub fn set_fps(fps: f32) {
    FPS.store(fps.max(1.0), Ordering::Release);
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Release);
}

pub fn paused() -> bool {
    PAUSED.load(Ordering::Acquire)
}

pub fn frame_duration() -> Duration {
    Duration::from_secs_f32(1.0 / fps())
}

pub fn average_fps() -> f32 {
    with_pacer(|pacer| pacer.average_fps())
}

pub fn advance_single_frame() {
    if paused() {
        with_pacer(|pacer| {
            pacer.force_render = true;
        });
    }
}

pub fn reset_timing(now: Instant) {
    with_pacer(|pacer| pacer.reset_timing(now));
}

pub fn tick(now: Instant) -> TickResult {
    with_pacer(|pacer| pacer.tick(now))
}

pub fn next_deadline() -> Instant {
    with_pacer(|pacer| pacer.next_deadline())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn init(now: Instant, fps_value: f32) {
        set_fps(fps_value);
        set_paused(false);
        set_frame_count(0);
        reset_timing(now);
    }

    #[test]
    #[serial]
    fn advances_on_full_interval() {
        let start = Instant::now();
        init(start, 60.0);

        let half = start + frame_duration() / 2;
        assert_eq!(tick(half), TickResult::default());

        let full = half + frame_duration() / 2;
        let t = tick(full);
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 1);
        assert_eq!(frame_count(), 1);
    }

    #[test]
    #[serial]
    fn catches_up_when_lagging() {
        let start = Instant::now();
        init(start, 30.0);

        let now = start + frame_duration() * 3;
        let t = tick(now);
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 3);
        assert_eq!(frame_count(), 3);
    }

    #[test]
    #[serial]
    fn pause_and_advance_single_frame() {
        let start = Instant::now();
        init(start, 60.0);
        set_paused(true);

        let later = start + Duration::from_secs(1);
        assert_eq!(tick(later), TickResult::default());
        assert_eq!(frame_count(), 0);

        advance_single_frame();
        let t = tick(later + Duration::from_millis(1));
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 1);
        assert_eq!(frame_count(), 1);
    }

    #[test]
    #[serial]
    fn applies_runtime_fps_changes() {
        let start = Instant::now();
        init(start, 60.0);

        let at_60hz = start + frame_duration();
        let t = tick(at_60hz);
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 1);

        set_fps(30.0);
        assert_eq!(fps(), 30.0);

        let partial_30hz = at_60hz + frame_duration() / 3;
        assert_eq!(tick(partial_30hz), TickResult::default());

        let full_30hz = at_60hz + frame_duration();
        let t = tick(full_30hz);
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 1);
        assert_eq!(frame_count(), 2);
    }
}
