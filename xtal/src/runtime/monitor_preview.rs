use std::sync::Arc;
use std::time::{Duration, Instant};

use log::warn;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::context::Context;

const MONITOR_PREVIEW_MAX_LONG_EDGE_PX: u32 = 640;
const MONITOR_PREVIEW_MAX_SHORT_EDGE_PX: u32 = 180;
const MONITOR_PREVIEW_MAX_FPS: f32 = 30.0;

struct MonitorBlitState {
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    shader: wgpu::ShaderModule,
    pipeline: Option<wgpu::RenderPipeline>,
    pipeline_format: Option<wgpu::TextureFormat>,
}

pub fn preview_size_for_main(
    main_width: u32,
    main_height: u32,
) -> PhysicalSize<u32> {
    let source_width = main_width.max(1);
    let source_height = main_height.max(1);
    let long_edge = source_width.max(source_height) as f32;
    let scale = (MONITOR_PREVIEW_MAX_LONG_EDGE_PX as f32 / long_edge).min(1.0);

    let mut width = (source_width as f32 * scale).round() as u32;
    let mut height = (source_height as f32 * scale).round() as u32;

    width = width.max(MONITOR_PREVIEW_MAX_SHORT_EDGE_PX.min(source_width));
    height = height.max(MONITOR_PREVIEW_MAX_SHORT_EDGE_PX.min(source_height));

    PhysicalSize::new(width.max(1), height.max(1))
}

impl MonitorBlitState {
    fn new(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("xtal-monitor-preview-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("xtal-monitor-preview-bind-group-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("xtal-monitor-preview-shader"),
                source: wgpu::ShaderSource::Wgsl(
                    MONITOR_PREVIEW_BLIT_WGSL.into(),
                ),
            });

        Self {
            sampler,
            bind_group_layout,
            shader,
            pipeline: None,
            pipeline_format: None,
        }
    }

    fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> &wgpu::RenderPipeline {
        if self.pipeline_format != Some(surface_format) {
            let pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("xtal-monitor-preview-pipeline-layout"),
                    bind_group_layouts: &[&self.bind_group_layout],
                    push_constant_ranges: &[],
                });

            let pipeline =
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("xtal-monitor-preview-pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &self.shader,
                        entry_point: Some("vs_main"),
                        compilation_options:
                            wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &self.shader,
                        entry_point: Some("fs_main"),
                        compilation_options:
                            wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

            self.pipeline = Some(pipeline);
            self.pipeline_format = Some(surface_format);
        }

        self.pipeline
            .as_ref()
            .expect("monitor preview pipeline should be initialized")
    }
}

pub enum RenderResult {
    Skipped,
    Rendered,
    OutOfMemory,
}

pub struct MonitorPreview {
    window: Arc<Window>,
    window_id: WindowId,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    blit_state: MonitorBlitState,
    last_draw: Instant,
}

impl MonitorPreview {
    pub fn create(
        event_loop: &ActiveEventLoop,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        initial_size: PhysicalSize<u32>,
    ) -> Result<Self, String> {
        let attrs = WindowAttributes::default()
            .with_title("Xtal Monitor Preview")
            .with_inner_size(LogicalSize::new(
                initial_size.width,
                initial_size.height,
            ))
            .with_visible(true);
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|err| err.to_string())?,
        );

        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| err.to_string())?;
        let caps = surface.get_capabilities(adapter);
        let format = choose_surface_format(&caps.formats)
            .ok_or_else(|| "monitor preview surface has no format".to_string())?;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: initial_size.width.max(1),
            height: initial_size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(device, &surface_config);

        Ok(Self {
            window_id: window.id(),
            window,
            surface,
            surface_config,
            blit_state: MonitorBlitState::new(device),
            last_draw: Instant::now() - Duration::from_secs(1),
        })
    }

    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn resize_to_main(
        &mut self,
        device: &wgpu::Device,
        main_width: u32,
        main_height: u32,
    ) {
        let width = main_width.max(1);
        let height = main_height.max(1);
        let _ = self.window.request_inner_size(PhysicalSize::new(width, height));
        self.resize_surface(device, width, height);
    }

    pub fn on_window_resized(
        &mut self,
        device: &wgpu::Device,
        new_size: PhysicalSize<u32>,
    ) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.resize_surface(device, new_size.width, new_size.height);
    }

    pub fn render_if_due(
        &mut self,
        context: &Context,
        source_texture: &wgpu::Texture,
        now: Instant,
    ) -> RenderResult {
        let min_interval =
            Duration::from_secs_f32(1.0 / MONITOR_PREVIEW_MAX_FPS);
        if now.duration_since(self.last_draw) < min_interval {
            return RenderResult::Skipped;
        }

        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface
                    .configure(context.device.as_ref(), &self.surface_config);
                return RenderResult::Skipped;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                warn!("monitor preview surface timeout while acquiring frame");
                return RenderResult::Skipped;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return RenderResult::OutOfMemory;
            }
            Err(wgpu::SurfaceError::Other) => {
                warn!("monitor preview surface error while acquiring frame");
                return RenderResult::Skipped;
            }
        };

        let output_view =
            output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let source_view =
            source_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("xtal-monitor-preview-bind-group"),
                layout: &self.blit_state.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(
                            &self.blit_state.sampler,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &source_view,
                        ),
                    },
                ],
            });
        let pipeline = self
            .blit_state
            .ensure_pipeline(context.device.as_ref(), self.surface_config.format);

        let source_size = source_texture.size();
        let (vx, vy, vw, vh) = fit_viewport(
            source_size.width.max(1),
            source_size.height.max(1),
            self.surface_config.width.max(1),
            self.surface_config.height.max(1),
        );

        let mut encoder =
            context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("xtal-monitor-preview-encoder"),
                });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("xtal-monitor-preview-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..4, 0..1);
        drop(pass);

        context.queue.submit(Some(encoder.finish()));
        output.present();
        self.last_draw = now;
        RenderResult::Rendered
    }

    fn resize_surface(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) {
        if self.surface_config.width == width && self.surface_config.height == height
        {
            return;
        }

        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(device, &self.surface_config);
    }
}

fn choose_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
        .or_else(|| formats.first().copied())
}

fn fit_viewport(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (f32, f32, f32, f32) {
    let source_width = source_width.max(1) as f32;
    let source_height = source_height.max(1) as f32;
    let target_width = target_width.max(1) as f32;
    let target_height = target_height.max(1) as f32;

    let scale = (target_width / source_width).min(target_height / source_height);
    let viewport_width = (source_width * scale).max(1.0);
    let viewport_height = (source_height * scale).max(1.0);
    let viewport_x = (target_width - viewport_width) * 0.5;
    let viewport_y = (target_height - viewport_height) * 0.5;

    (viewport_x, viewport_y, viewport_width, viewport_height)
}

const MONITOR_PREVIEW_BLIT_WGSL: &str = r#"
@group(0) @binding(0)
var tex_sampler: sampler;

@group(0) @binding(1)
var tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2f, 4>(
        vec2f(-1.0, -1.0),
        vec2f(1.0, -1.0),
        vec2f(-1.0, 1.0),
        vec2f(1.0, 1.0),
    );

    let p = positions[vertex_index];
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * vec2f(0.5, 0.5) + vec2f(0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let uv = vec2f(in.uv.x, 1.0 - in.uv.y);
    return textureSample(tex, tex_sampler, uv);
}
"#;
