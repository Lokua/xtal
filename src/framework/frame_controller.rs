use nannou::prelude::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::logging::*;

pub struct FrameController {
    #[allow(dead_code)]
    fps: f32,
    frame_duration: Duration,
    /// Captured every update call regardless if the frame is skipped or rendered
    last_frame_time: Instant,
    last_render_time: Instant,
    accumulator: Duration,
    frame_count: u32,
    render_flag: bool,
    paused: bool,
}

impl FrameController {
    pub fn new(fps: f32) -> Self {
        let now = Instant::now();
        Self {
            fps,
            frame_duration: Duration::from_secs_f32(1.0 / fps),
            last_frame_time: now,
            last_render_time: now,
            accumulator: Duration::ZERO,
            frame_count: 0,
            render_flag: false,
            paused: false,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_frame_time;
        self.accumulator += elapsed;
        self.last_frame_time = now;
        self.render_flag = false;

        // Render frames for each interval the accumulator surpasses
        while self.accumulator >= self.frame_duration {
            self.accumulator -= self.frame_duration;
            self.frame_count += 1;
            self.render_flag = true;
        }

        // Adjust for small drifts (if the drift is negligible, round up to the next frame)
        if self.accumulator < Duration::from_millis(1) {
            self.accumulator = Duration::ZERO;
        }

        if self.render_flag {
            trace!(
                "Rendering. Time since last render: {:?} (expected: {:?})",
                now - self.last_render_time,
                self.frame_duration
            );
            self.last_render_time = now;
        } else {
            trace!(
                "Skipping render this frame. Time since last frame: {:?}",
                elapsed
            );
        }
    }

    pub fn should_render(&self) -> bool {
        self.render_flag && !self.paused
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn reset_frame_count(&mut self) {
        self.frame_count = 0;
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}

static CONTROLLER: Lazy<Mutex<Option<FrameController>>> =
    Lazy::new(|| Mutex::new(None));

pub fn ensure_controller(fps: f32) {
    let mut controller = CONTROLLER.lock().unwrap();
    if controller.is_none() {
        *controller = Some(FrameController::new(fps));
    }
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
        let mut controller = CONTROLLER.lock().unwrap();
        if let Some(controller) = controller.as_mut() {
            controller.update();
            controller.should_render()
        } else {
            false
        }
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
    let should_render = {
        let controller = CONTROLLER.lock().unwrap();
        controller.as_ref().map_or(false, |c| c.should_render())
    };

    if should_render {
        view_fn(app, model, frame);
    }

    should_render
}

pub fn frame_count() -> u32 {
    let controller = CONTROLLER.lock().unwrap();
    controller.as_ref().map_or(0, |c| c.frame_count())
}

pub fn reset_frame_count() {
    let mut controller = CONTROLLER.lock().unwrap();
    if let Some(controller) = controller.as_mut() {
        controller.reset_frame_count();
    }
}

pub fn set_frame_count(count: u32) {
    let mut controller = CONTROLLER.lock().unwrap();
    if let Some(controller) = controller.as_mut() {
        controller.frame_count = count;
    } else {
        warn!("Cannot set frame_count: FrameController is not initialized.");
    }
}

pub fn fps() -> f32 {
    let controller = CONTROLLER.lock().unwrap();
    controller.as_ref().map_or(0.0, |c| c.fps())
}

pub fn set_fps(fps: f32) {
    let mut controller = CONTROLLER.lock().unwrap();
    if let Some(controller) = controller.as_mut() {
        controller.fps = fps;
        controller.frame_duration = Duration::from_secs_f32(1.0 / fps);
    } else {
        warn!("Cannot set fps: FrameController is not initialized.");
    }
}

pub fn is_paused() -> bool {
    let controller = CONTROLLER.lock().unwrap();
    controller.as_ref().map_or(false, |c| c.is_paused())
}

pub fn set_paused(paused: bool) {
    let mut controller = CONTROLLER.lock().unwrap();
    if let Some(controller) = controller.as_mut() {
        controller.set_paused(paused);
    } else {
        warn!("Unable to paused frame_controller");
    }
}
