use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use crate::framework::util::AtomicF32;

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static FPS: AtomicF32 = AtomicF32::new(60.0);
static PAUSED: AtomicBool = AtomicBool::new(false);

pub fn frame_count() -> u32 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

pub fn set_frame_count(count: u32) {
    FRAME_COUNT.store(count, Ordering::Relaxed);
}

pub fn reset_frame_count() {
    set_frame_count(0);
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
    fps()
}
