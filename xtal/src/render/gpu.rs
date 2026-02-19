use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use log::{error, info, warn};
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use wgpu::util::DeviceExt;

use crate::frame::Frame;
use crate::graph::{
    ComputeNodeSpec, GraphSpec, NodeSpec, RenderNodeSpec, RenderRead,
    RenderTarget, ResourceDecl, ResourceHandle, ResourceKind, TextureHandle,
};
use crate::mesh::{Mesh, MeshVertexKind};
use crate::shader_watch::ShaderWatch;
use crate::uniforms::UniformBanks;

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub fn compute_row_padding(unpadded_bytes_per_row: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let rem = unpadded_bytes_per_row % align;
    if rem == 0 { 0 } else { align - rem }
}

pub struct CompiledGraph {
    surface_format: wgpu::TextureFormat,
    present_source: PresentSource,
    nodes: Vec<CompiledNode>,
    offscreen_resource_ids: Vec<TextureHandle>,
    offscreen_textures: HashMap<TextureHandle, GpuTexture>,
    image_textures: HashMap<TextureHandle, GpuTexture>,
    texture_labels: HashMap<TextureHandle, String>,
}

struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
    format: wgpu::TextureFormat,
}

enum CompiledNode {
    Render(RenderNode),
    Compute(ComputeNode),
}

struct RenderNode {
    name: String,
    target: RenderTarget,
    sampled_reads: Vec<TextureHandle>,
    pass: RenderPass,
}

struct ComputeNode {
    name: String,
    target: TextureHandle,
    pass: ComputePass,
}

#[derive(Clone, Copy)]
enum PresentSource {
    Surface,
    Texture(TextureHandle),
}

struct RenderPass {
    shader_path: PathBuf,
    target_format: wgpu::TextureFormat,
    mesh_kind: MeshVertexKind,
    render_pipeline: wgpu::RenderPipeline,
    meshes: Vec<MeshDraw>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    sampler: Option<wgpu::Sampler>,
    watcher: Option<ShaderWatch>,
}

struct MeshDraw {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

struct ComputePass {
    shader_path: PathBuf,
    compute_pipeline: wgpu::ComputePipeline,
    storage_bind_group_layout: wgpu::BindGroupLayout,
    watcher: Option<ShaderWatch>,
}

impl CompiledGraph {
    pub fn compile(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        graph: GraphSpec,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let present_source_handle = find_present_source(&graph)?;
        let (offscreen_resource_ids, image_resources, texture_labels) =
            collect_texture_resources(&graph.resources);

        validate_graph_resources(
            &graph,
            &offscreen_resource_ids,
            &image_resources,
            present_source_handle,
        )?;

        let mut nodes = Vec::new();

        for node in graph.nodes {
            match node {
                NodeSpec::Render(render) => {
                    let sampled_reads = render
                        .reads
                        .iter()
                        .filter_map(|resource| match resource {
                            RenderRead::Texture(texture) => Some(*texture),
                            RenderRead::Uniform(_) => None,
                        })
                        .collect::<Vec<_>>();

                    let target_format = match render.write {
                        RenderTarget::Surface => surface_format,
                        RenderTarget::Texture(_) => OFFSCREEN_FORMAT,
                    };

                    let pass = RenderPass::new(
                        device,
                        target_format,
                        &render,
                        &sampled_reads,
                        uniform_layout,
                    )?;

                    nodes.push(CompiledNode::Render(RenderNode {
                        name: render.name,
                        target: render.write,
                        sampled_reads,
                        pass,
                    }));
                }
                NodeSpec::Compute(compute) => {
                    let pass =
                        ComputePass::new(device, &compute, uniform_layout)?;

                    nodes.push(CompiledNode::Compute(ComputeNode {
                        name: compute.name,
                        target: compute.read_write,
                        pass,
                    }));
                }
                NodeSpec::Present { .. } => {}
            }
        }

        if nodes.is_empty() {
            return Err("graph has no executable nodes".to_string());
        }

        let mut image_textures = HashMap::new();

        for (handle, path) in image_resources {
            let label = texture_labels
                .get(&handle)
                .map(|name| name.as_str())
                .unwrap_or("xtal-image-texture");
            let texture = load_image_texture(device, queue, label, &path)?;
            image_textures.insert(handle, texture);
        }

        Ok(Self {
            surface_format,
            present_source: if let Some(source) = present_source_handle {
                PresentSource::Texture(source)
            } else {
                PresentSource::Surface
            },
            nodes,
            offscreen_resource_ids,
            offscreen_textures: HashMap::new(),
            image_textures,
            texture_labels,
        })
    }

    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        frame: &mut Frame,
        uniforms: &UniformBanks,
        surface_size: [u32; 2],
    ) -> Result<(), String> {
        self.ensure_offscreen_textures(device, surface_size);

        for node in &mut self.nodes {
            match node {
                CompiledNode::Render(node) => {
                    node.pass.update_if_changed(
                        device,
                        &node.sampled_reads,
                        uniforms.bind_group_layout(),
                    );

                    let texture_bind_group = if !node.sampled_reads.is_empty() {
                        Some(node.pass.create_texture_bind_group(
                            device,
                            &self.offscreen_textures,
                            &self.image_textures,
                            &node.sampled_reads,
                        )?)
                    } else {
                        None
                    };

                    let target_view = match node.target {
                        RenderTarget::Surface => frame.surface_view.clone(),
                        RenderTarget::Texture(texture) => self
                            .offscreen_textures
                            .get(&texture)
                            .ok_or_else(|| {
                                format!(
                                    "render target '{}' was not declared as texture2d",
                                    texture_label(texture, &self.texture_labels)
                                )
                            })?
                            .view
                            .clone(),
                    };

                    let mut render_pass = frame.encoder().begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some(&node.name),
                            color_attachments: &[Some(
                                wgpu::RenderPassColorAttachment {
                                    view: &target_view,
                                    resolve_target: None,
                                    depth_slice: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(
                                            wgpu::Color::BLACK,
                                        ),
                                        store: wgpu::StoreOp::Store,
                                    },
                                },
                            )],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        },
                    );

                    render_pass.set_pipeline(&node.pass.render_pipeline);
                    render_pass.set_bind_group(0, uniforms.bind_group(), &[]);

                    if let Some(bind_group) = texture_bind_group.as_ref() {
                        render_pass.set_bind_group(1, bind_group, &[]);
                    }

                    for mesh in &node.pass.meshes {
                        render_pass
                            .set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        render_pass.draw(0..mesh.vertex_count, 0..1);
                    }
                }
                CompiledNode::Compute(node) => {
                    node.pass.update_if_changed(
                        device,
                        uniforms.bind_group_layout(),
                    );

                    let storage_bind_group =
                        node.pass.create_storage_bind_group(
                            device,
                            &self.offscreen_textures,
                            &node.target,
                        )?;

                    let width = surface_size[0].max(1);
                    let height = surface_size[1].max(1);
                    let workgroup_x = width.div_ceil(8);
                    let workgroup_y = height.div_ceil(8);

                    let mut compute_pass = frame.encoder().begin_compute_pass(
                        &wgpu::ComputePassDescriptor {
                            label: Some(&node.name),
                            timestamp_writes: None,
                        },
                    );

                    compute_pass.set_pipeline(&node.pass.compute_pipeline);
                    compute_pass.set_bind_group(0, uniforms.bind_group(), &[]);
                    compute_pass.set_bind_group(1, &storage_bind_group, &[]);
                    compute_pass.dispatch_workgroups(
                        workgroup_x,
                        workgroup_y,
                        1,
                    );
                }
            }
        }

        if let PresentSource::Texture(source) = self.present_source {
            let source_view = if let Some(texture) =
                self.offscreen_textures.get(&source)
            {
                texture.view.clone()
            } else if let Some(texture) = self.image_textures.get(&source) {
                texture.view.clone()
            } else {
                return Err(format!(
                    "present source '{}' is not a known texture resource",
                    texture_label(source, &self.texture_labels)
                ));
            };

            blit_texture_to_surface(
                device,
                frame,
                &source_view,
                self.surface_format,
            );
        }

        Ok(())
    }

    fn ensure_offscreen_textures(
        &mut self,
        device: &wgpu::Device,
        size: [u32; 2],
    ) {
        let width = size[0].max(1);
        let height = size[1].max(1);

        for handle in &self.offscreen_resource_ids {
            let needs_new = self
                .offscreen_textures
                .get(handle)
                .is_none_or(|texture| texture.size != [width, height]);

            if !needs_new {
                continue;
            }

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(texture_label(*handle, &self.texture_labels)),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OFFSCREEN_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            let view =
                texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.offscreen_textures.insert(
                *handle,
                GpuTexture {
                    texture,
                    view,
                    size: [width, height],
                    format: OFFSCREEN_FORMAT,
                },
            );
        }
    }

    pub fn recording_source_texture(&self) -> Option<&wgpu::Texture> {
        match self.present_source {
            PresentSource::Surface => None,
            PresentSource::Texture(source) => self
                .offscreen_textures
                .get(&source)
                .map(|texture| &texture.texture)
                .or_else(|| {
                    self.image_textures
                        .get(&source)
                        .map(|texture| &texture.texture)
                }),
        }
    }

    pub fn recording_source_format(&self) -> Option<wgpu::TextureFormat> {
        match self.present_source {
            PresentSource::Surface => None,
            PresentSource::Texture(source) => self
                .offscreen_textures
                .get(&source)
                .map(|texture| texture.format)
                .or_else(|| {
                    self.image_textures
                        .get(&source)
                        .map(|texture| texture.format)
                }),
        }
    }
}

impl RenderPass {
    fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        node: &RenderNodeSpec,
        sampled_reads: &[TextureHandle],
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let shader_path = normalize_shader_path(&node.shader_path)?;

        if !node
            .reads
            .iter()
            .any(|resource| matches!(resource, RenderRead::Uniform(_)))
        {
            return Err(format!(
                "render node '{}' must read 'params'",
                node.name
            ));
        }

        let source = fs::read_to_string(&shader_path).map_err(|err| {
            format!(
                "failed to read shader '{}': {}",
                shader_path.display(),
                err
            )
        })?;

        validate_shader(&source).map_err(|err| {
            format!(
                "shader validation failed for '{}': {}",
                shader_path.display(),
                err
            )
        })?;

        let (texture_bind_group_layout, sampler) = if sampled_reads.is_empty() {
            (None, None)
        } else {
            let layout =
                create_texture_bind_group_layout(device, sampled_reads.len());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("xtal-texture-sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            (Some(layout), Some(sampler))
        };

        let mesh_kind = infer_mesh_kind_for_node(node)?;
        let render_pipeline = create_render_pipeline(
            device,
            target_format,
            mesh_kind,
            uniform_layout,
            texture_bind_group_layout.as_ref(),
            &source,
            &node.name,
        );
        let meshes = node
            .meshes
            .iter()
            .map(|mesh| create_mesh_draw(device, mesh))
            .collect::<Vec<_>>();

        let watcher = match ShaderWatch::start(shader_path.clone()) {
            Ok(watch) => Some(watch),
            Err(err) => {
                warn!(
                    "shader watch unavailable for '{}': {}",
                    shader_path.display(),
                    err
                );
                None
            }
        };

        Ok(Self {
            shader_path,
            target_format,
            mesh_kind,
            render_pipeline,
            meshes,
            texture_bind_group_layout,
            sampler,
            watcher,
        })
    }

    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        offscreen_textures: &HashMap<TextureHandle, GpuTexture>,
        image_textures: &HashMap<TextureHandle, GpuTexture>,
        sampled_reads: &[TextureHandle],
    ) -> Result<wgpu::BindGroup, String> {
        let layout =
            self.texture_bind_group_layout.as_ref().ok_or_else(|| {
                "texture bind group layout missing for sampled pass".to_string()
            })?;

        let sampler = self
            .sampler
            .as_ref()
            .ok_or_else(|| "sampler missing for sampled pass".to_string())?;

        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Sampler(sampler),
        }];

        for (index, handle) in sampled_reads.iter().enumerate() {
            let view = if let Some(texture) = offscreen_textures.get(handle) {
                &texture.view
            } else if let Some(texture) = image_textures.get(handle) {
                &texture.view
            } else {
                return Err(format!(
                    "texture resource '{}' is not available",
                    handle.index()
                ));
            };

            entries.push(wgpu::BindGroupEntry {
                binding: (index + 1) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("xtal-texture-bind-group"),
            layout,
            entries: &entries,
        }))
    }

    fn update_if_changed(
        &mut self,
        device: &wgpu::Device,
        sampled_reads: &[TextureHandle],
        uniform_layout: &wgpu::BindGroupLayout,
    ) {
        if !self.watcher.as_ref().is_some_and(ShaderWatch::take_changed) {
            return;
        }

        info!("reloading shader: {}", self.shader_path.display());

        let source = match fs::read_to_string(&self.shader_path) {
            Ok(source) => source,
            Err(err) => {
                error!(
                    "failed to read shader '{}': {}",
                    self.shader_path.display(),
                    err
                );
                return;
            }
        };

        if let Err(err) = validate_shader(&source) {
            error!(
                "shader validation failed for '{}': {}",
                self.shader_path.display(),
                err
            );
            return;
        }

        self.render_pipeline = create_render_pipeline(
            device,
            self.target_format,
            self.mesh_kind,
            uniform_layout,
            self.texture_bind_group_layout.as_ref(),
            &source,
            "xtal-hot-reloaded",
        );

        if !sampled_reads.is_empty() && self.texture_bind_group_layout.is_none()
        {
            warn!(
                "shader '{}' reads textures but no texture bind group layout is configured",
                self.shader_path.display()
            );
        }

        info!("shader reload applied: {}", self.shader_path.display());
    }
}

impl ComputePass {
    fn new(
        device: &wgpu::Device,
        node: &ComputeNodeSpec,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let shader_path = normalize_shader_path(&node.shader_path)?;

        let source = fs::read_to_string(&shader_path).map_err(|err| {
            format!(
                "failed to read compute shader '{}': {}",
                shader_path.display(),
                err
            )
        })?;

        validate_shader(&source).map_err(|err| {
            format!(
                "compute shader validation failed for '{}': {}",
                shader_path.display(),
                err
            )
        })?;

        let storage_bind_group_layout =
            create_storage_bind_group_layout(device);

        let compute_pipeline = create_compute_pipeline(
            device,
            uniform_layout,
            &storage_bind_group_layout,
            &source,
            &node.name,
        );

        let watcher = match ShaderWatch::start(shader_path.clone()) {
            Ok(watch) => Some(watch),
            Err(err) => {
                warn!(
                    "compute shader watch unavailable for '{}': {}",
                    shader_path.display(),
                    err
                );
                None
            }
        };

        Ok(Self {
            shader_path,
            compute_pipeline,
            storage_bind_group_layout,
            watcher,
        })
    }

    fn create_storage_bind_group(
        &self,
        device: &wgpu::Device,
        textures: &HashMap<TextureHandle, GpuTexture>,
        target: &TextureHandle,
    ) -> Result<wgpu::BindGroup, String> {
        let texture = textures.get(target).ok_or_else(|| {
            format!(
                "compute target '{}' is not a declared offscreen texture",
                target.index()
            )
        })?;

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("xtal-compute-storage-bind-group"),
            layout: &self.storage_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            }],
        }))
    }

    fn update_if_changed(
        &mut self,
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
    ) {
        if !self.watcher.as_ref().is_some_and(ShaderWatch::take_changed) {
            return;
        }

        info!("reloading compute shader: {}", self.shader_path.display());

        let source = match fs::read_to_string(&self.shader_path) {
            Ok(source) => source,
            Err(err) => {
                error!(
                    "failed to read compute shader '{}': {}",
                    self.shader_path.display(),
                    err
                );
                return;
            }
        };

        if let Err(err) = validate_shader(&source) {
            error!(
                "compute shader validation failed for '{}': {}",
                self.shader_path.display(),
                err
            );
            return;
        }

        self.compute_pipeline = create_compute_pipeline(
            device,
            uniform_layout,
            &self.storage_bind_group_layout,
            &source,
            "xtal-hot-reloaded-compute",
        );

        info!(
            "compute shader reload applied: {}",
            self.shader_path.display()
        );
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    mesh_kind: MeshVertexKind,
    uniform_layout: &wgpu::BindGroupLayout,
    texture_layout: Option<&wgpu::BindGroupLayout>,
    source: &str,
    label: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });

    let bind_group_layouts = if let Some(texture_layout) = texture_layout {
        vec![uniform_layout, texture_layout]
    } else {
        vec![uniform_layout]
    };

    let layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("xtal-pipeline-layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

    let vertex_buffers = [vertex_buffer_layout_for_kind(mesh_kind)];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("xtal-render-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

const POSITION_2D_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] =
    wgpu::vertex_attr_array![0 => Float32x2];
const POSITION_3D_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] =
    wgpu::vertex_attr_array![0 => Float32x3];

fn vertex_buffer_layout_for_kind(
    mesh_kind: MeshVertexKind,
) -> wgpu::VertexBufferLayout<'static> {
    match mesh_kind {
        MeshVertexKind::Position2D => wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_2D_VERTEX_ATTRIBUTES,
        },
        MeshVertexKind::Position3D => wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_3D_VERTEX_ATTRIBUTES,
        },
    }
}

fn create_mesh_draw(
    device: &wgpu::Device,
    mesh: &Mesh,
) -> MeshDraw {
    match mesh {
        Mesh::Positions2D(vertices) => {
            let buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("xtal-mesh-2d-vertices"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            MeshDraw {
                vertex_buffer: buffer,
                vertex_count: mesh.vertex_count(),
            }
        }
        Mesh::Positions3D(vertices) => {
            let buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("xtal-mesh-3d-vertices"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            MeshDraw {
                vertex_buffer: buffer,
                vertex_count: mesh.vertex_count(),
            }
        }
    }
}

fn infer_mesh_kind_for_node(
    node: &RenderNodeSpec,
) -> Result<MeshVertexKind, String> {
    let Some(first_mesh) = node.meshes.first() else {
        return Err(format!("render node '{}' has no meshes", node.name));
    };

    let mesh_kind = first_mesh.vertex_kind();
    for (index, mesh) in node.meshes.iter().enumerate() {
        if mesh.vertex_kind() != mesh_kind {
            return Err(format!(
                "render node '{}' has mixed mesh vertex kinds; mesh {} differs from first mesh",
                node.name, index
            ));
        }
    }

    Ok(mesh_kind)
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    uniform_layout: &wgpu::BindGroupLayout,
    storage_layout: &wgpu::BindGroupLayout,
    source: &str,
    label: &str,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });

    let layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("xtal-compute-pipeline-layout"),
            bind_group_layouts: &[uniform_layout, storage_layout],
            push_constant_ranges: &[],
        });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("xtal-compute-pipeline"),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn create_texture_bind_group_layout(
    device: &wgpu::Device,
    texture_count: usize,
) -> wgpu::BindGroupLayout {
    let mut entries = Vec::with_capacity(texture_count + 1);

    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    });

    for index in 0..texture_count {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: (index + 1) as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float {
                    filterable: true,
                },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("xtal-texture-bind-group-layout"),
        entries: &entries,
    })
}

fn create_storage_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("xtal-compute-storage-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: OFFSCREEN_FORMAT,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }],
    })
}

fn blit_texture_to_surface(
    device: &wgpu::Device,
    frame: &mut Frame,
    source_view: &wgpu::TextureView,
    surface_format: wgpu::TextureFormat,
) {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("xtal-present-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let bind_group_layout = create_texture_bind_group_layout(device, 1);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("xtal-present-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("xtal-present-shader"),
        source: wgpu::ShaderSource::Wgsl(PRESENT_BLIT_WGSL.into()),
    });

    let pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("xtal-present-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("xtal-present-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(
                ),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(
                ),
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

    let surface_view = frame.surface_view.clone();
    let mut render_pass =
        frame
            .encoder()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("xtal-present-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
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

    render_pass.set_pipeline(&pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..4, 0..1);
}

const PRESENT_BLIT_WGSL: &str = r#"
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
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, in.uv);
}
"#;

fn load_image_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    name: &str,
    path: &Path,
) -> Result<GpuTexture, String> {
    let resolved = normalize_shader_path(path)?;
    let bytes = fs::read(&resolved).map_err(|err| {
        format!(
            "failed to read image '{}' at '{}': {}",
            name,
            resolved.display(),
            err
        )
    })?;

    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|err| {
        format!(
            "failed to decode PNG '{}' at '{}': {}",
            name,
            resolved.display(),
            err
        )
    })?;
    let output_buffer_size = reader.output_buffer_size().ok_or_else(|| {
        format!(
            "failed to determine PNG output buffer size for '{}' at '{}'",
            name,
            resolved.display()
        )
    })?;
    let mut buf = vec![0; output_buffer_size];
    let info = reader.next_frame(&mut buf).map_err(|err| {
        format!(
            "failed to read PNG frame '{}' at '{}': {}",
            name,
            resolved.display(),
            err
        )
    })?;
    let src = &buf[..info.buffer_size()];
    let width = info.width.max(1);
    let height = info.height.max(1);

    let rgba = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => src.to_vec(),
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            let mut out = Vec::with_capacity((width * height * 4) as usize);
            for pixel in src.chunks_exact(3) {
                out.push(pixel[0]);
                out.push(pixel[1]);
                out.push(pixel[2]);
                out.push(255);
            }
            out
        }
        _ => {
            return Err(format!(
                "unsupported PNG format for '{}': {:?} {:?} (expected RGB/RGBA 8-bit)",
                name, info.color_type, info.bit_depth
            ));
        }
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(name),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: IMAGE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    Ok(GpuTexture {
        texture,
        view,
        size: [width, height],
        format: IMAGE_FORMAT,
    })
}

fn validate_shader(source: &str) -> Result<(), String> {
    let module = wgsl::parse_str(source).map_err(|err| err.to_string())?;

    let mut validator =
        Validator::new(ValidationFlags::all(), Capabilities::all());

    validator
        .validate(&module)
        .map_err(|err| err.to_string())
        .map(|_| ())
}

fn normalize_shader_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to get current directory: {}", err))?;

    Ok(cwd.join(path))
}

fn find_present_source(
    graph: &GraphSpec,
) -> Result<Option<TextureHandle>, String> {
    let mut source = None;

    for node in &graph.nodes {
        if let NodeSpec::Present { source: candidate } = node {
            if source.is_some() {
                return Err("graph can only have one Present node".to_string());
            }
            source = Some(*candidate);
        }
    }

    Ok(source)
}

fn collect_texture_resources(
    resources: &[ResourceDecl],
) -> (
    Vec<TextureHandle>,
    HashMap<TextureHandle, PathBuf>,
    HashMap<TextureHandle, String>,
) {
    let mut offscreen = Vec::new();
    let mut images = HashMap::new();
    let mut labels = HashMap::new();

    for resource in resources {
        let ResourceHandle::Texture(handle) = resource.handle else {
            continue;
        };

        labels.insert(handle, resource.name.clone());

        match &resource.kind {
            ResourceKind::Texture2d => offscreen.push(handle),
            ResourceKind::Image2d { path } => {
                images.insert(handle, path.clone());
            }
            ResourceKind::Uniforms => unreachable!(),
        }
    }

    (offscreen, images, labels)
}

fn validate_graph_resources(
    graph: &GraphSpec,
    offscreen_resource_ids: &[TextureHandle],
    image_resources: &HashMap<TextureHandle, PathBuf>,
    present_source: Option<TextureHandle>,
) -> Result<(), String> {
    let offscreen_ids = offscreen_resource_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let image_ids = image_resources.keys().copied().collect::<HashSet<_>>();

    if let Some(source) = present_source {
        if !offscreen_ids.contains(&source) && !image_ids.contains(&source) {
            return Err(format!(
                "present source texture {} is not a declared offscreen/image texture resource",
                source.index()
            ));
        }
    }

    for node in &graph.nodes {
        match node {
            NodeSpec::Render(render) => {
                if let RenderTarget::Texture(target) = render.write {
                    if !offscreen_ids.contains(&target) {
                        return Err(format!(
                            "render node '{}' writes texture {} which is not a declared texture2d resource",
                            render.name,
                            target.index()
                        ));
                    }
                }

                for read in &render.reads {
                    if let RenderRead::Texture(texture) = read {
                        if !offscreen_ids.contains(texture)
                            && !image_ids.contains(texture)
                        {
                            return Err(format!(
                                "render node '{}' reads texture {} which is not a declared texture2d/image resource",
                                render.name,
                                texture.index()
                            ));
                        }
                    }
                }
            }
            NodeSpec::Compute(compute) => {
                if !offscreen_ids.contains(&compute.read_write) {
                    return Err(format!(
                        "compute node '{}' read_write target '{}' is not a declared texture2d resource",
                        compute.name,
                        compute.read_write.index()
                    ));
                }
            }
            NodeSpec::Present { .. } => {}
        }
    }

    Ok(())
}

fn texture_label(
    handle: TextureHandle,
    labels: &HashMap<TextureHandle, String>,
) -> &str {
    labels
        .get(&handle)
        .map(String::as_str)
        .unwrap_or("texture")
}
