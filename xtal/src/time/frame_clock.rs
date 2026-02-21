use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::core::util::AtomicF32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TickResult {
    pub should_render: bool,
    pub frames_advanced: u32,
}

struct Pacer {
    last_tick: Instant,
    accumulator: Duration,
    transport_origin: Instant,
    transport_offset: Duration,
    transport_paused_at: Option<Instant>,
    transport_paused_total: Duration,
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
            transport_origin: now,
            transport_offset: Duration::ZERO,
            transport_paused_at: None,
            transport_paused_total: Duration::ZERO,
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
        // Re-anchor to avoid very large monotonic deltas while preserving
        // current elapsed transport time exactly.
        let elapsed = self.transport_elapsed(now);
        self.transport_origin = now;
        self.transport_offset = elapsed;
        self.transport_paused_total = Duration::ZERO;
        self.transport_paused_at = if paused() { Some(now) } else { None };
        self.publish_transport_elapsed_at(now);
    }

    fn tick(&mut self, now: Instant) -> TickResult {
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        self.accumulator += elapsed;
        let is_paused = paused();
        self.publish_transport_elapsed_at(now);

        if self.force_render {
            self.force_render = false;
            if is_paused {
                self.transport_offset += frame_duration();
                self.publish_transport_elapsed_at(now);
            }
            advance_frames(1);
            self.record_render(now);
            return TickResult {
                should_render: true,
                frames_advanced: 1,
            };
        }

        if is_paused {
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

    fn set_paused(&mut self, paused: bool, now: Instant) {
        match (self.transport_paused_at, paused) {
            (None, true) => {
                self.transport_paused_at = Some(now);
            }
            (Some(started_at), false) => {
                self.transport_paused_total +=
                    now.saturating_duration_since(started_at);
                self.transport_paused_at = None;
            }
            _ => {}
        }
        self.publish_transport_elapsed_at(now);
    }

    fn set_transport_elapsed(&mut self, now: Instant, elapsed: Duration) {
        self.transport_origin = now;
        self.transport_offset = elapsed;
        self.transport_paused_total = Duration::ZERO;
        self.transport_paused_at = if paused() { Some(now) } else { None };
        self.publish_transport_elapsed_at(now);
    }

    fn transport_elapsed(&self, now: Instant) -> Duration {
        let effective_now = self.transport_paused_at.unwrap_or(now);
        let since_origin =
            effective_now.saturating_duration_since(self.transport_origin);
        let running = since_origin.saturating_sub(self.transport_paused_total);
        self.transport_offset + running
    }

    fn publish_transport_elapsed_at(&self, now: Instant) {
        TRANSPORT_ELAPSED_SECONDS
            .store(self.transport_elapsed(now).as_secs_f32(), Ordering::Release);
    }
}

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static FPS: AtomicF32 = AtomicF32::new(60.0);
static PAUSED: AtomicBool = AtomicBool::new(false);
static TRANSPORT_ELAPSED_SECONDS: AtomicF32 = AtomicF32::new(0.0);
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
    set_elapsed_seconds(0.0);
    reset_timing(Instant::now());
}

pub fn fps() -> f32 {
    FPS.load(Ordering::Acquire)
}

pub fn set_fps(fps: f32) {
    FPS.store(fps.max(1.0), Ordering::Release);
}

fn set_paused_at(paused: bool, now: Instant) {
    PAUSED.store(paused, Ordering::Release);
    with_pacer(|pacer| pacer.set_paused(paused, now));
}

pub fn set_paused(paused: bool) {
    set_paused_at(paused, Instant::now());
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

pub fn elapsed_seconds() -> f32 {
    with_pacer(|pacer| {
        let now = Instant::now();
        let elapsed = pacer.transport_elapsed(now).as_secs_f32();
        TRANSPORT_ELAPSED_SECONDS.store(elapsed, Ordering::Release);
        elapsed
    })
}

pub fn set_elapsed_seconds(seconds: f32) {
    let seconds = seconds.max(0.0);
    with_pacer(|pacer| {
        pacer.set_transport_elapsed(
            Instant::now(),
            Duration::from_secs_f32(seconds),
        );
    });
}

#[cfg(test)]
fn elapsed_seconds_at(now: Instant) -> f32 {
    with_pacer(|pacer| pacer.transport_elapsed(now).as_secs_f32())
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
        set_paused_at(false, now);
        set_frame_count(0);
        set_elapsed_seconds(0.0);
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
        set_paused_at(true, start);

        let later = start + Duration::from_secs(1);
        assert_eq!(tick(later), TickResult::default());
        assert_eq!(frame_count(), 0);

        advance_single_frame();
        let t = tick(later + Duration::from_millis(1));
        assert!(t.should_render);
        assert_eq!(t.frames_advanced, 1);
        assert_eq!(frame_count(), 1);
        assert!(elapsed_seconds_at(later + Duration::from_millis(1)) > 0.0);
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

    #[test]
    #[serial]
    fn transport_elapsed_tracks_monotonic_time_when_running() {
        let start = Instant::now();
        init(start, 60.0);

        let later = start + Duration::from_millis(150);
        let _ = tick(later);
        assert!((elapsed_seconds_at(later) - 0.15).abs() < 0.000_1);
    }

    #[test]
    #[serial]
    fn transport_elapsed_does_not_advance_while_paused() {
        let start = Instant::now();
        init(start, 60.0);

        let paused_at = start + Duration::from_millis(100);
        let _ = tick(paused_at);
        let before_pause = elapsed_seconds_at(paused_at);
        set_paused_at(true, paused_at);
        let _ = tick(start + Duration::from_millis(600));
        assert!(
            (elapsed_seconds_at(start + Duration::from_millis(600))
                - before_pause)
                .abs()
                < 0.000_1
        );
    }
}
