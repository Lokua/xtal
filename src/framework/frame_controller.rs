//! Provides a hand-rolled frame rate / counting singleton for syncing video
//! recording, animations, and rendering with nannou. Nannou never implemented
//! frame rate so here we are. The implementation is technically flawed but so
//! far has been working well enough for my purposes (animations are tight and
//! videos seem perfectly synced). The module is meant for internal
//! framework/runtime use and should not be interacted with directly.

#[cfg(not(feature = "atomic_frames"))]
pub use lock_impl::*;

#[cfg(not(feature = "atomic_frames"))]
mod lock_impl {
    use nannou::prelude::*;
    use once_cell::sync::Lazy;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    use crate::framework::prelude::*;

    static CONTROLLER: Lazy<RwLock<FrameController>> =
        Lazy::new(|| RwLock::new(FrameController::new(60.0)));

    pub fn ensure_controller(fps: f32) {
        let mut controller = CONTROLLER.write();
        controller.fps = fps;
        controller.frame_duration = Duration::from_secs_f32(1.0 / fps);
    }

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
            controller.should_render()
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
        let should_render = CONTROLLER.read().should_render();

        if should_render {
            view_fn(app, model, frame);
        }

        should_render
    }

    pub fn frame_count() -> u32 {
        CONTROLLER.read().frame_count()
    }

    pub fn reset_frame_count() {
        CONTROLLER.write().reset_frame_count();
    }

    pub fn set_frame_count(count: u32) {
        CONTROLLER
            .write()
            .frame_count
            .store(count, Ordering::Relaxed);
    }

    pub fn fps() -> f32 {
        CONTROLLER.read().fps()
    }

    pub fn set_fps(fps: f32) {
        let mut controller = CONTROLLER.write();
        controller.fps = fps;
        controller.frame_duration = Duration::from_secs_f32(1.0 / fps);
    }

    pub fn is_paused() -> bool {
        CONTROLLER.read().is_paused()
    }

    pub fn set_paused(paused: bool) {
        CONTROLLER.write().set_paused(paused);
    }

    pub fn average_fps() -> f32 {
        CONTROLLER.read().average_fps()
    }

    pub fn advance_single_frame() {
        let mut controller = CONTROLLER.write();
        if controller.paused {
            controller.force_render = true;
        }
    }

    pub fn clear_force_render() {
        CONTROLLER.write().force_render = false;
    }

    struct FrameController {
        #[allow(dead_code)]
        fps: f32,
        frame_duration: Duration,
        last_frame_time: Instant,
        last_render_time: Instant,
        accumulator: Duration,
        frame_count: AtomicU32,
        render_flag: bool,
        force_render: bool,
        paused: bool,
        frame_intervals: Vec<Duration>,
        max_intervals: usize,
    }

    impl FrameController {
        fn new(fps: f32) -> Self {
            info!("Using lock_impl");
            let now = Instant::now();
            Self {
                fps,
                frame_duration: Duration::from_secs_f32(1.0 / fps),
                last_frame_time: now,
                last_render_time: now,
                accumulator: Duration::ZERO,
                frame_count: AtomicU32::new(0),
                render_flag: false,
                force_render: false,
                paused: false,
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
            self.render_flag = false;

            if self.force_render {
                self.frame_count.fetch_add(1, Ordering::Relaxed);
                trace!("Forced frame increment");
                return;
            }

            if !self.paused {
                // Render frames for each interval the accumulator surpasses
                while self.accumulator >= self.frame_duration {
                    self.accumulator -= self.frame_duration;
                    self.frame_count.fetch_add(1, Ordering::Relaxed);
                    self.render_flag = true;
                }

                // Adjust for small drifts (do we really need this?)
                if self.accumulator < Duration::from_millis(1) {
                    self.accumulator = Duration::ZERO;
                }
            }

            if self.render_flag {
                let render_interval = now - self.last_render_time;
                self.frame_intervals.push(render_interval);
                if self.frame_intervals.len() > self.max_intervals {
                    self.frame_intervals.remove(0);
                }
                trace!(
                    "Rendering. frame_count: {}. Time since last render: {:.2?} (expected: {:.2?})",
                    self.frame_count(),
                    now - self.last_render_time,
                    self.frame_duration
                );
                self.last_render_time = now;
            } else {
                trace!(
                    "Skipping render this frame. Time since last frame: {:.2?}",
                    elapsed
                );
            }
        }

        fn should_render(&self) -> bool {
            self.force_render || (!self.paused && self.render_flag)
        }

        fn frame_count(&self) -> u32 {
            self.frame_count.load(Ordering::Relaxed)
        }

        fn reset_frame_count(&mut self) {
            self.frame_count.store(0, Ordering::Relaxed);
        }

        fn fps(&self) -> f32 {
            self.fps
        }

        fn is_paused(&self) -> bool {
            self.paused
        }

        fn set_paused(&mut self, paused: bool) {
            self.paused = paused;
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

        #[test]
        fn test_frame_pacing() {
            let clock = MockClock::new();
            let mut controller = FrameController::new(60.0);
            controller.last_frame_time = clock.now();
            controller.last_render_time = clock.now();

            // Simulate exactly one frame worth of time
            clock.advance(controller.frame_duration);
            controller.update_with_time(clock.now());
            assert_eq!(controller.frame_count(), 1);
            assert!(controller.should_render());

            // Simulate half a frame - should not increment
            clock.advance(controller.frame_duration / 2);
            controller.update_with_time(clock.now());
            assert_eq!(controller.frame_count(), 1);
            assert!(!controller.should_render());

            // Simulate the next half - should increment
            clock.advance(controller.frame_duration / 2);
            controller.update_with_time(clock.now());
            assert_eq!(controller.frame_count(), 2);
            assert!(controller.should_render());
        }

        #[test]
        fn test_lag() {
            let clock = MockClock::new();
            let mut controller = FrameController::new(60.0);
            controller.last_frame_time = clock.now();
            controller.last_render_time = clock.now();

            // Simulate exactly one frame worth of time
            clock.advance(controller.frame_duration);
            controller.update_with_time(clock.now());
            assert_eq!(controller.frame_count(), 1);
            assert!(controller.should_render());

            // Simulate being a second
            clock.advance(controller.frame_duration * 3);
            controller.update_with_time(clock.now());
            assert_eq!(controller.frame_count(), 4);
            assert!(controller.should_render());
        }
    }
}

#[cfg(feature = "atomic_frames")]
pub use atomic_impl::*;

#[cfg(feature = "atomic_frames")]
mod atomic_impl {
    use nannou::prelude::*;
    use once_cell::sync::Lazy;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    use crate::framework::prelude::*;

    static CONTROLLER: Lazy<RwLock<FrameController>> =
        Lazy::new(|| RwLock::new(FrameController::new()));

    // Atomics used to lessen the amount of CONTROLLER locks
    static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
    static FPS: AtomicF32 = AtomicF32::new(60.0);
    static RENDER_FLAG: AtomicBool = AtomicBool::new(false);
    static FORCE_RENDER: AtomicBool = AtomicBool::new(false);
    static PAUSED: AtomicBool = AtomicBool::new(false);

    // Keeping for backwards compat - should replace with set_fps
    pub fn ensure_controller(fps: f32) {
        FPS.store(fps, Ordering::Release);
    }

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

    pub fn is_paused() -> bool {
        PAUSED.load(Ordering::Relaxed)
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
            info!("Using atomic_impl");
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
}
