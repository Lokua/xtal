use std::sync::Arc;
use std::time::Instant;

/// Shared GPU and frame state exposed to advanced sketch hooks.
pub struct RuntimeContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    window_size: [u32; 2],
    render_size: [u32; 2],
    scale_factor: f64,
    frame_count: u64,
    start_time: Instant,
}

impl RuntimeContext {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        window_size: [u32; 2],
        scale_factor: f64,
    ) -> Self {
        Self {
            device,
            queue,
            window_size,
            render_size: window_size,
            scale_factor,
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn set_window_size(&mut self, window_size: [u32; 2]) {
        self.window_size = window_size;
        self.render_size = window_size;
    }

    pub fn set_render_size(&mut self, render_size: [u32; 2]) {
        self.render_size = [render_size[0].max(1), render_size[1].max(1)];
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub fn resolution(&self) -> [f32; 2] {
        [self.render_size[0] as f32, self.render_size[1] as f32]
    }

    pub fn resolution_u32(&self) -> [u32; 2] {
        self.render_size
    }

    pub fn window_size_u32(&self) -> [u32; 2] {
        self.window_size
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn next_frame(&mut self) {
        self.frame_count += 1;
    }
}
