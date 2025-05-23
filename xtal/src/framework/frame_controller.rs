//! Provides a hand-rolled frame rate / counting singleton for syncing video
//! recording, animations, and rendering with nannou. Nannou never implemented
//! frame rate so here we are. The implementation is technically flawed but so
//! far has been working well enough for my purposes (animations are tight and
//! videos seem perfectly synced). The module is meant for internal
//! framework/runtime use and should not be interacted with directly.

use nannou::prelude::*;
use parking_lot::RwLock;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use crate::framework::prelude::*;

static CONTROLLER: LazyLock<RwLock<FrameController>> =
    LazyLock::new(|| RwLock::new(FrameController::new()));

// Atomics used to lessen the amount of CONTROLLER locks
static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static FPS: AtomicF32 = AtomicF32::new(60.0);
static RENDER_FLAG: AtomicBool = AtomicBool::new(false);
static FORCE_RENDER: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);

pub fn wrapped_update<M, F>(
    app: &App,
    model: &mut M,
    update: Update,
    update_fn: F,
) where
    F: FnOnce(&App, &mut M, Update),
{
    let should_update = {
        let mut controller = CONTROLLER.write();
        controller.update();
        should_render()
    };

    if should_update {
        update_fn(app, model, update);
    }
}

pub fn wrapped_view<M, F>(
    app: &App,
    model: &M,
    frame: Frame,
    view_fn: F,
) -> bool
where
    F: FnOnce(&App, &M, Frame),
{
    let do_render = should_render();

    if do_render {
        view_fn(app, model, frame);
    }

    do_render
}

fn should_render() -> bool {
    FORCE_RENDER.load(Ordering::Acquire)
        || (!PAUSED.load(Ordering::Acquire)
            && RENDER_FLAG.load(Ordering::Acquire))
}

pub fn frame_count() -> u32 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

pub fn reset_frame_count() {
    set_frame_count(0);
}

pub fn set_frame_count(count: u32) {
    FRAME_COUNT.store(count, Ordering::Relaxed);
}

pub fn fps() -> f32 {
    FPS.load(Ordering::Acquire)
}

pub fn set_fps(fps: f32) {
    FPS.store(fps, Ordering::Release);
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

pub fn average_fps() -> f32 {
    CONTROLLER.read().average_fps()
}

pub fn advance_single_frame() {
    if PAUSED.load(Ordering::Acquire) {
        FORCE_RENDER.store(true, Ordering::Release);
    }
}

pub fn clear_force_render() {
    FORCE_RENDER.store(false, Ordering::Release);
}

pub fn frame_duration() -> Duration {
    Duration::from_secs_f32(1.0 / FPS.load(Ordering::Relaxed))
}

struct FrameController {
    last_frame_time: Instant,
    last_render_time: Instant,
    accumulator: Duration,
    frame_intervals: Vec<Duration>,
    max_intervals: usize,
}

impl FrameController {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last_frame_time: now,
            last_render_time: now,
            accumulator: Duration::ZERO,
            frame_intervals: Vec::new(),
            max_intervals: 90,
        }
    }

    fn update(&mut self) {
        self.update_with_time(Instant::now());
    }

    fn update_with_time(&mut self, now: Instant) {
        let elapsed = now - self.last_frame_time;
        self.accumulator += elapsed;
        self.last_frame_time = now;
        let frame_duration = frame_duration();
        RENDER_FLAG.store(false, Ordering::Release);

        if FORCE_RENDER.load(Ordering::Relaxed) {
            FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
            trace!("Forced frame increment");
            return;
        }

        if !PAUSED.load(Ordering::Acquire) {
            // Render frames for each interval the accumulator surpasses
            while self.accumulator >= frame_duration {
                self.accumulator -= frame_duration;
                FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
                RENDER_FLAG.store(true, Ordering::Relaxed);
            }

            // Adjust for small drifts (do we really need this?)
            if self.accumulator < Duration::from_millis(1) {
                self.accumulator = Duration::ZERO;
            }
        }

        if RENDER_FLAG.load(Ordering::Acquire) {
            let render_interval = now - self.last_render_time;
            self.frame_intervals.push(render_interval);
            if self.frame_intervals.len() > self.max_intervals {
                self.frame_intervals.remove(0);
            }
            trace!(
                "Rendering. frame_count: {}. \
                        Time since last render: {:.2?} (expected: {:.2?})",
                self.frame_count(),
                now - self.last_render_time,
                frame_duration
            );
            self.last_render_time = now;
        } else {
            trace!(
                "Skipping render this frame. Time since last frame: {:.2?}",
                elapsed
            );
        }
    }

    fn frame_count(&self) -> u32 {
        FRAME_COUNT.load(Ordering::Relaxed)
    }

    fn average_fps(&self) -> f32 {
        if self.frame_intervals.is_empty() {
            return 0.0;
        }
        let sum: Duration = self.frame_intervals.iter().copied().sum();
        let avg = sum / self.frame_intervals.len() as u32;
        1.0 / avg.as_secs_f32()
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Mutex;

    use serial_test::serial;

    use super::*;

    struct MockClock {
        current_time: Mutex<Instant>,
    }

    impl MockClock {
        fn new() -> Self {
            Self {
                current_time: Mutex::new(Instant::now()),
            }
        }

        fn advance(&self, duration: Duration) {
            let mut time = self.current_time.lock().unwrap();
            *time += duration;
        }

        fn now(&self) -> Instant {
            *self.current_time.lock().unwrap()
        }
    }

    fn init() {
        FRAME_COUNT.store(0, Ordering::SeqCst);
        RENDER_FLAG.store(false, Ordering::SeqCst);
        FORCE_RENDER.store(false, Ordering::SeqCst);
        PAUSED.store(false, Ordering::SeqCst);
    }

    #[test]
    #[serial]
    fn test_frame_pacing() {
        init();
        let clock = MockClock::new();
        let mut controller = FrameController::new();
        controller.last_frame_time = clock.now();
        controller.last_render_time = clock.now();

        // Simulate exactly one frame worth of time
        clock.advance(frame_duration());
        controller.update_with_time(clock.now());
        assert_eq!(controller.frame_count(), 1);
        assert!(should_render());

        // Simulate half a frame - should not increment
        clock.advance(frame_duration() / 2);
        controller.update_with_time(clock.now());
        assert_eq!(controller.frame_count(), 1);
        assert!(!should_render());

        // Simulate the next half - should increment
        clock.advance(frame_duration() / 2);
        controller.update_with_time(clock.now());
        assert_eq!(controller.frame_count(), 2);
        assert!(should_render());
    }

    #[test]
    #[serial]
    fn test_lag() {
        init();
        let clock = MockClock::new();
        let mut controller = FrameController::new();
        controller.last_frame_time = clock.now();
        controller.last_render_time = clock.now();

        // Simulate exactly one frame worth of time
        clock.advance(frame_duration());
        controller.update_with_time(clock.now());
        assert_eq!(controller.frame_count(), 1);
        assert!(should_render());

        // Simulate being seconds ahead of time
        clock.advance(frame_duration() * 3);
        controller.update_with_time(clock.now());
        assert_eq!(controller.frame_count(), 4);
        assert!(should_render());
    }
}
