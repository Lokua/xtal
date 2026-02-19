use std::sync::Arc;

pub struct Frame {
    pub surface_view: wgpu::TextureView,
    encoder: Option<wgpu::CommandEncoder>,
    output: Option<wgpu::SurfaceTexture>,
    queue: Arc<wgpu::Queue>,
}

impl Frame {
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

    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("frame command encoder already submitted")
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self
            .output
            .as_ref()
            .expect("frame output texture already presented")
            .texture
    }

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
