use bevy_reflect::{Reflect, TypeInfo, Typed};
use bytemuck::{Pod, Zeroable};
use nannou::prelude::*;

use super::prelude::*;

pub struct GpuState<V: Pod + Zeroable> {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    params_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    n_vertices: u32,
    _marker: std::marker::PhantomData<V>,
}

impl<V: Pod + Zeroable + Typed> GpuState<V> {
    pub fn new<P: Pod + Zeroable>(
        app: &App,
        shader: wgpu::ShaderModuleDescriptor,
        params: &P,
        vertices: Option<&[V]>,
        topology: wgpu::PrimitiveTopology,
        blend: Option<wgpu::BlendState>,
    ) -> Self {
        let window = app.main_window();
        let device = window.device();
        let sample_count = window.msaa_samples();
        let format = Frame::TEXTURE_FORMAT;
        let shader_module = device.create_shader_module(shader);

        let params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params Buffer"),
                contents: bytemuck::bytes_of(params),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
            });

        let params_bind_group_layout =
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
            });

        let params_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &params_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                }],
                label: Some("Params Bind Group"),
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&params_bind_group_layout],
                push_constant_ranges: &[],
            });

        let (vertex_buffer, n_vertices) = if let Some(verts) = vertices {
            let buffer = Self::create_vertex_buffer(device, verts);
            (Some(buffer), verts.len() as u32)
        } else {
            (None, 0)
        };

        let vertex_attributes = if vertices.is_some() {
            Self::infer_vertex_attributes()
        } else {
            vec![]
        };

        let vertex_buffers = if vertices.is_some() {
            vec![wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<V>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }]
        } else {
            vec![]
        };

        let render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        Self {
            render_pipeline,
            vertex_buffer,
            params_buffer,
            params_bind_group,
            n_vertices,
            _marker: std::marker::PhantomData,
        }
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

    pub fn update_params<P: Pod>(&self, app: &App, params: &P) {
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
            // Why don't we need this?
            // self.n_vertices = vertices.len() as u32;
        }

        window.queue().write_buffer(
            self.vertex_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(vertices),
        );
    }

    pub fn update<P: Pod>(&mut self, app: &App, params: &P, vertices: &[V]) {
        self.update_params(app, params);
        self.update_vertex_buffer(app, vertices);
    }

    pub fn render(&self, frame: &Frame) {
        let mut encoder = frame.command_encoder();
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| {
                color.load_op(wgpu::LoadOp::Load)
            })
            .begin(&mut encoder);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.params_bind_group, &[]);

        if let Some(ref vertex_buffer) = self.vertex_buffer {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..self.n_vertices, 0..1);
        } else {
            error!("Use render_procedural if not using a vertex buffer");
            panic!();
        }
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
                        println!("Field: {} -> {:?}", field, field_info);

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

impl GpuState<BasicPositionVertex> {
    pub fn new_full_screen<P: Pod + Zeroable>(
        app: &App,
        shader: wgpu::ShaderModuleDescriptor,
        params: &P,
    ) -> Self {
        Self::new(
            app,
            shader,
            params,
            Some(QUAD_COVER_VERTICES),
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        )
    }
}

/// Specialized implementation for procedural rendering,
/// when there is no VertexInput in shader
impl GpuState<()> {
    pub fn new_procedural<P: Pod + Zeroable>(
        app: &App,
        shader: wgpu::ShaderModuleDescriptor,
        params: &P,
    ) -> Self {
        Self::new(
            app,
            shader,
            params,
            None,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
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
