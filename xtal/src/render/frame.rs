//! Per-frame surface and command submission wrapper.
//!
//! The runtime acquires a `wgpu::SurfaceTexture`, wraps it in `Frame`, passes
//! the mutable command encoder through graph execution and capture code, then
//! consumes the frame with `submit`. Consuming the frame makes presentation
//! explicit and prevents later access to an encoder that has already been
//! finished.

use std::sync::Arc;

/// One acquired surface texture plus its command encoder.
///
/// `Frame` is intentionally single-use. Callers borrow the encoder while
/// recording GPU work, then call `submit` to finish the command buffer and
/// present the surface texture.
pub struct Frame {
    /// View of the surface texture used when rendering directly to the window.
    pub surface_view: wgpu::TextureView,
    encoder: Option<wgpu::CommandEncoder>,
    output: Option<wgpu::SurfaceTexture>,
    queue: Arc<wgpu::Queue>,
}

impl Frame {
    /// Creates a frame around a freshly acquired surface texture.
    pub fn new(
        device: &wgpu::Device,
        queue: Arc<wgpu::Queue>,
        output: wgpu::SurfaceTexture,
    ) -> Self {
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("xtal-frame-encoder"),
            });

        Self {
            surface_view,
            encoder: Some(encoder),
            output: Some(output),
            queue,
        }
    }

    /// Returns the command encoder used to record this frame's GPU work.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("frame command encoder already submitted")
    }

    /// Returns the underlying surface texture for copy/readback operations.
    pub fn output_texture(&self) -> &wgpu::Texture {
        &self
            .output
            .as_ref()
            .expect("frame output texture already presented")
            .texture
    }

    /// Borrows the encoder and surface texture together.
    ///
    /// This avoids split-borrow friction at call sites that need to record a
    /// copy from the surface texture into the same command encoder.
    pub fn encoder_and_output_texture(
        &mut self,
    ) -> (&mut wgpu::CommandEncoder, &wgpu::Texture) {
        let encoder = self
            .encoder
            .as_mut()
            .expect("frame command encoder already submitted");
        let texture = &self
            .output
            .as_ref()
            .expect("frame output texture already presented")
            .texture;
        (encoder, texture)
    }

    /// Finishes GPU command recording, submits work, and presents the output.
    pub fn submit(mut self) -> wgpu::SubmissionIndex {
        let encoder = self
            .encoder
            .take()
            .expect("frame command encoder already submitted");

        let submission_index = self.queue.submit(Some(encoder.finish()));

        if let Some(output) = self.output.take() {
            output.present();
        }

        submission_index
    }
}
