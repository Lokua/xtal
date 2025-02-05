use bevy_reflect::{Reflect, TypeInfo, Typed};
use bytemuck::{Pod, Zeroable};
use naga;
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use nannou::prelude::*;
use nannou::wgpu;
use notify::{Event, RecursiveMode, Watcher};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::prelude::*;

struct PipelineCreationState<'a> {
    device: &'a wgpu::Device,
    shader_module: &'a wgpu::ShaderModule,
    pipeline_layout: &'a wgpu::PipelineLayout,
    vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    sample_count: u32,
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

pub struct GpuState<V: Pod + Zeroable> {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    params_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    n_vertices: u32,
    depth_texture: Option<wgpu::TextureView>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
    params_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'static>>,
    sample_count: u32,
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    _marker: std::marker::PhantomData<V>,

    // Shaded access for hot reloading
    update_state: Arc<Mutex<Option<PathBuf>>>,
    _watcher: Option<notify::RecommendedWatcher>,
}

impl<V: Pod + Zeroable + Typed> GpuState<V> {
    /// Create a GpuState instance that accepts your own vertices.
    /// Example at `src/sketches/genuary_2025/g25_18_wind.rs`.
    /// See the specialized `new_procedural` and `new_full_screen` constructors
    /// for easier to get up and running shaders.
    pub fn new<P: Pod + Zeroable>(
        app: &App,
        shader_path: PathBuf,
        params: &P,
        vertices: Option<&[V]>,
        topology: wgpu::PrimitiveTopology,
        blend: Option<wgpu::BlendState>,
        enable_depth_testing: bool,
        watch: bool,
    ) -> Self {
        let shader_content = fs::read_to_string(&shader_path)
            .expect("Failed to read shader file");

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Hot Reloadable Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_content.into()),
        };

        let update_state = Arc::new(Mutex::new(None));
        let watcher = if watch {
            Some(Self::setup_shader_watcher(
                shader_path.clone(),
                update_state.clone(),
            ))
        } else {
            None
        };

        let window = app.main_window();
        let device = window.device();
        let sample_count = window.msaa_samples();
        let rect = window.rect();
        let format = Frame::TEXTURE_FORMAT;
        let shader_module = device.create_shader_module(shader);

        let params_bind_group_layout =
            Self::create_params_bind_group_layout::<P>(device);
        let params_buffer = Self::create_params_buffer(device, params);
        let params_bind_group = Self::create_params_bind_group(
            device,
            &params_bind_group_layout,
            &params_buffer,
        );

        let texture_bind_group_layout =
            Self::create_texture_bind_group_layout(device);

        let dummy_texture = wgpu::TextureBuilder::new()
            .size([1, 1])
            .format(Frame::TEXTURE_FORMAT)
            .dimension(wgpu::TextureDimension::D2)
            .usage(
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .sample_count(1)
            .build(device);

        let dummy_view = dummy_texture.view().build();

        let dummy_texture_sampler =
            device.create_sampler(&wgpu::SamplerDescriptor::default());

        let texture_bind_group = wgpu::BindGroupBuilder::new()
            .texture_view(&dummy_view)
            .sampler(&dummy_texture_sampler)
            .build(device, &texture_bind_group_layout);

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[
                    &params_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let (vertex_buffer, n_vertices) = if let Some(verts) = vertices {
            let buffer = Self::create_vertex_buffer(device, verts);
            (Some(buffer), verts.len() as u32)
        } else {
            (None, 0)
        };

        let vertex_buffers = if vertices.is_some() {
            vec![Self::create_vertex_buffer_layout()]
        } else {
            vec![]
        };

        let depth_stencil = if enable_depth_testing {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };

        let depth_texture = if enable_depth_testing {
            let texture = wgpu::TextureBuilder::new()
                // Hack: multiply by two since rect isn't pixel_depth aware
                .size([rect.w() as u32 * 2, rect.h() as u32 * 2])
                .format(wgpu::TextureFormat::Depth32Float)
                .usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
                .sample_count(sample_count)
                .build(device);

            Some(texture.view().build())
        } else {
            None
        };

        let creation_state = PipelineCreationState {
            device,
            shader_module: &shader_module,
            pipeline_layout: &pipeline_layout,
            vertex_buffers: &vertex_buffers,
            sample_count,
            format,
            topology,
            blend,
            depth_stencil: depth_stencil.clone(),
        };

        let render_pipeline = Self::create_render_pipeline(creation_state);

        Self {
            render_pipeline,
            vertex_buffer,
            params_buffer,
            params_bind_group,
            n_vertices,
            depth_stencil,
            depth_texture,
            _marker: std::marker::PhantomData,
            topology,
            blend,
            params_bind_group_layout,
            vertex_buffers,
            sample_count,
            texture_bind_group,
            texture_bind_group_layout,
            update_state,
            _watcher: watcher,
        }
    }

    fn create_params_bind_group_layout<P: Pod>(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX
                    | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<P>() as _,
                    ),
                },
                count: None,
            }],
            label: Some("Params Bind Group Layout"),
        })
    }

    fn create_params_buffer<P: Pod>(
        device: &wgpu::Device,
        params: &P,
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::bytes_of(params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_params_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Params Bind Group"),
        })
    }

    fn create_vertex_buffer(
        device: &wgpu::Device,
        vertices: &[V],
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        let vertex_attributes = Self::infer_vertex_attributes();
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<V>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vertex_attributes
                .into_iter()
                .collect::<Vec<_>>()
                .leak(),
        }
    }

    fn create_texture_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        wgpu::BindGroupLayoutBuilder::new()
            .texture(
                wgpu::ShaderStages::FRAGMENT,
                false,
                wgpu::TextureViewDimension::D2,
                wgpu::TextureSampleType::Float { filterable: true },
            )
            .sampler(wgpu::ShaderStages::FRAGMENT, true)
            .build(device)
    }

    fn create_render_pipeline(
        state: PipelineCreationState,
    ) -> wgpu::RenderPipeline {
        state
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(state.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: state.shader_module,
                    entry_point: "vs_main",
                    buffers: state.vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: state.shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: state.format,
                        blend: state.blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: state.topology,
                    ..Default::default()
                },
                depth_stencil: state.depth_stencil,
                multisample: wgpu::MultisampleState {
                    count: state.sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }

    fn setup_shader_watcher(
        path: PathBuf,
        state: Arc<Mutex<Option<PathBuf>>>,
    ) -> notify::RecommendedWatcher {
        let path_to_watch = path.clone();

        let mut watcher = notify::recommended_watcher(move |res| {
            let event: Event = match res {
                Ok(event) => event,
                Err(_) => return,
            };

            if event.kind
                != notify::EventKind::Modify(notify::event::ModifyKind::Data(
                    notify::event::DataChange::Content,
                ))
            {
                return;
            }

            trace!("Shader {:?} changed", path);
            if let Ok(mut guard) = state.lock() {
                *guard = Some(path.clone());
            }
        })
        .expect("Failed to create watcher");

        watcher
            .watch(&path_to_watch, RecursiveMode::NonRecursive)
            .expect("Failed to start watching shader file");

        watcher
    }

    pub fn set_input_texture(
        &mut self,
        app: &App,
        texture_view: &wgpu::TextureView,
    ) {
        let window = app.main_window();
        let device = window.device();

        let sampler =
            device.create_sampler(&wgpu::SamplerDescriptor::default());

        self.texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            texture_view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("Texture Bind Group"),
            });
    }

    /// This will be called multiple times when we update but it doesn't matter
    /// since the update_state's content will be none due to `guard.take()`
    fn update_shader(&mut self, app: &App) {
        if let Ok(mut guard) = self.update_state.lock() {
            if let Some(path) = guard.take() {
                info!("Reloading shader from {:?}", path);

                if let Ok(shader_content) = fs::read_to_string(&path) {
                    let parse_result = wgsl::parse_str(&shader_content);

                    if let Ok(module) = parse_result {
                        let mut validator = Validator::new(
                            ValidationFlags::all(),
                            Capabilities::empty(),
                        );

                        let validation_result = validator.validate(&module);
                        if let Err(validation_error) = validation_result {
                            error!(
                                "Shader validation failed:\n{:?}",
                                validation_error
                            );
                            return;
                        }

                        // If we got here, shader is valid - create the module
                        let shader = wgpu::ShaderModuleDescriptor {
                            label: Some("Hot Reloadable Shader"),
                            source: wgpu::ShaderSource::Wgsl(
                                shader_content.into(),
                            ),
                        };

                        let window = app.main_window();
                        let device = window.device();

                        let shader_module = device.create_shader_module(shader);
                        let pipeline_layout = device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("Pipeline Layout"),
                                bind_group_layouts: &[
                                    &self.params_bind_group_layout
                                ],
                                push_constant_ranges: &[],
                            },
                        );

                        let creation_state = PipelineCreationState {
                            device,
                            shader_module: &shader_module,
                            pipeline_layout: &pipeline_layout,
                            vertex_buffers: &self.vertex_buffers,
                            sample_count: self.sample_count,
                            format: Frame::TEXTURE_FORMAT,
                            topology: self.topology,
                            blend: self.blend,
                            depth_stencil: self.depth_stencil.clone(),
                        };

                        self.render_pipeline =
                            Self::create_render_pipeline(creation_state);

                        info!("Shader pipeline successfully recreated");
                    } else {
                        error!("Failed to parse shader: {:?}", parse_result);
                    }
                }
            }
        }
    }

    /// For non-procedural and full-screen shaders when vertices are altered on CPU
    pub fn update<P: Pod>(&mut self, app: &App, params: &P, vertices: &[V]) {
        self.update_shader(app);
        self.update_params(app, params);
        self.update_vertex_buffer(app, vertices);
    }

    /// For procedural and full-screen shaders that do not need updated vertices
    pub fn update_params<P: Pod>(&mut self, app: &App, params: &P) {
        self.update_shader(app);
        app.main_window().queue().write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(params),
        );
    }

    pub fn update_vertex_buffer(&mut self, app: &App, vertices: &[V]) {
        let window = app.main_window();
        let device = window.device();

        if vertices.len() as u32 != self.n_vertices {
            if let Some(_) = self.vertex_buffer {
                self.vertex_buffer =
                    Some(Self::create_vertex_buffer(device, vertices));
            }

            // Not sure why this isn't 100% needed.
            // This works with or without it, but it's not even correct
            // as it doesn't multiply the length by the actual position data
            // e.g. len * 6 for quads:
            //
            // self.n_vertices = vertices.len() as u32;
        }

        window.queue().write_buffer(
            self.vertex_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(vertices),
        );
    }

    pub fn render(&self, frame: &Frame) {
        let mut encoder = frame.command_encoder();

        let mut render_pass_builder = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| {
                color.load_op(wgpu::LoadOp::Load)
            });

        if let Some(ref depth_texture) = self.depth_texture {
            render_pass_builder = render_pass_builder
                .depth_stencil_attachment(depth_texture, |depth| depth);
        }

        let mut render_pass = render_pass_builder.begin(&mut encoder);

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.params_bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);

        if let Some(ref vertex_buffer) = self.vertex_buffer {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..self.n_vertices, 0..1);
        } else {
            error!("Use render_procedural if not using a vertex buffer");
            panic!();
        }
    }

    pub fn render_to_texture(&self, app: &App) -> wgpu::TextureView {
        let window = app.main_window();
        let device = window.device();

        // Create multisampled texture for rendering
        let msaa_texture = wgpu::TextureBuilder::new()
            .size([window.inner_size_pixels().0, window.inner_size_pixels().1])
            .format(Frame::TEXTURE_FORMAT)
            .dimension(wgpu::TextureDimension::D2)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
            .sample_count(self.sample_count)
            .build(device);

        // Create non-multisampled texture for resolving and sampling
        let resolve_texture = wgpu::TextureBuilder::new()
            .size([window.inner_size_pixels().0, window.inner_size_pixels().1])
            .format(Frame::TEXTURE_FORMAT)
            .dimension(wgpu::TextureDimension::D2)
            .usage(
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .sample_count(1)
            .build(device);

        let msaa_view = msaa_texture.view().build();
        let resolve_view = resolve_texture.view().build();

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render to Texture Encoder"),
            });

        {
            let mut render_pass = wgpu::RenderPassBuilder::new()
                .color_attachment(&msaa_view, |color| {
                    color
                        .load_op(wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))
                        .store_op(true)
                        .resolve_target(Some(&resolve_view))
                })
                .begin(&mut encoder);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.params_bind_group, &[]);
            render_pass.set_bind_group(1, &self.texture_bind_group, &[]);

            if let Some(ref vertex_buffer) = self.vertex_buffer {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..self.n_vertices, 0..1);
            } else {
                render_pass.draw(0..3, 0..1);
            }
        }

        window.queue().submit(std::iter::once(encoder.finish()));

        resolve_view
    }

    fn infer_vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        let mut attributes = Vec::new();
        let mut offset = 0;

        match V::type_info() {
            TypeInfo::Struct(struct_info) => {
                for (i, field) in
                    struct_info.field_names().into_iter().enumerate()
                {
                    if let Some(field_info) = struct_info.field(field) {
                        trace!("Field: {} -> {:?}", field, field_info);

                        let format = match field_info.type_path() {
                            "f32" => wgpu::VertexFormat::Float32,
                            "[f32; 2]" => wgpu::VertexFormat::Float32x2,
                            "[f32; 3]" => wgpu::VertexFormat::Float32x3,
                            "[f32; 4]" => wgpu::VertexFormat::Float32x4,
                            t => {
                                error!("Unsupported vertex field type: {}", t);
                                panic!();
                            }
                        };

                        attributes.push(wgpu::VertexAttribute {
                            offset: offset as u64,
                            shader_location: i as u32,
                            format,
                        });

                        offset += match format {
                            wgpu::VertexFormat::Float32 => 4,
                            wgpu::VertexFormat::Float32x2 => 8,
                            wgpu::VertexFormat::Float32x3 => 12,
                            wgpu::VertexFormat::Float32x4 => 16,
                            _ => unreachable!(),
                        };
                    }
                }
            }
            _ => {
                error!("Type must be a struct");
                panic!();
            }
        }

        attributes
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Reflect)]
pub struct BasicPositionVertex {
    pub position: [f32; 2],
}

pub const QUAD_COVER_VERTICES: &[BasicPositionVertex] = &[
    BasicPositionVertex {
        position: [-1.0, -1.0],
    },
    BasicPositionVertex {
        position: [1.0, -1.0],
    },
    BasicPositionVertex {
        position: [-1.0, 1.0],
    },
    BasicPositionVertex {
        position: [1.0, -1.0],
    },
    BasicPositionVertex {
        position: [1.0, 1.0],
    },
    BasicPositionVertex {
        position: [-1.0, 1.0],
    },
];

/// Specialized impl for shaders that simply need every pixel.
/// See interference and wave_fract for examples.
impl GpuState<BasicPositionVertex> {
    pub fn new_full_screen<P: Pod + Zeroable>(
        app: &App,
        shader_path: PathBuf,
        params: &P,
        watch: bool,
    ) -> Self {
        Self::new(
            app,
            shader_path,
            params,
            Some(QUAD_COVER_VERTICES),
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            watch,
        )
    }
}

/// Specialized impl for purly procedural shaders (no vertices).
/// See spiral.rs for an example.
impl GpuState<()> {
    pub fn new_procedural<P: Pod + Zeroable>(
        app: &App,
        shader_path: PathBuf,
        params: &P,
        watch: bool,
    ) -> Self {
        Self::new(
            app,
            shader_path,
            params,
            None,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            watch,
        )
    }

    pub fn render_procedural(&self, frame: &Frame, vertex_count: u32) {
        let mut encoder = frame.command_encoder();
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| {
                color.load_op(wgpu::LoadOp::Load)
            })
            .begin(&mut encoder);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.params_bind_group, &[]);
        render_pass.draw(0..vertex_count, 0..1);
    }
}
