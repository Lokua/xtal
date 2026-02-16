use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use log::{error, info, warn};
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};

use crate::frame::Frame;
use crate::graph::{
    ComputeNodeSpec, GraphSpec, NodeSpec, RenderNodeSpec, ResourceDecl,
    ResourceKind,
};
use crate::shader_watch::ShaderWatch;
use crate::uniforms::UniformBanks;

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct CompiledGraph {
    present_source: String,
    nodes: Vec<CompiledNode>,
    texture_resource_names: Vec<String>,
    textures: HashMap<String, OffscreenTexture>,
}

struct OffscreenTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
}

enum CompiledNode {
    Render(RenderNode),
    Compute(ComputeNode),
}

struct RenderNode {
    name: String,
    target: String,
    sampled_reads: Vec<String>,
    pass: RenderPass,
}

struct ComputeNode {
    name: String,
    target: String,
    pass: ComputePass,
}

struct RenderPass {
    shader_path: PathBuf,
    target_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    sampler: Option<wgpu::Sampler>,
    watcher: Option<ShaderWatch>,
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
        surface_format: wgpu::TextureFormat,
        graph: GraphSpec,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let present_source = find_present_source(&graph)?;
        let texture_resource_names =
            collect_texture_resources(&graph.resources);

        validate_graph_resources(
            &graph,
            &texture_resource_names,
            &present_source,
        )?;

        let mut nodes = Vec::new();

        for node in graph.nodes {
            match node {
                NodeSpec::Render(render) => {
                    let sampled_reads = render
                        .reads
                        .iter()
                        .filter(|resource| resource.as_str() != "params")
                        .cloned()
                        .collect::<Vec<_>>();

                    let target_format = if render.write == "surface" {
                        surface_format
                    } else {
                        OFFSCREEN_FORMAT
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

        Ok(Self {
            present_source,
            nodes,
            texture_resource_names,
            textures: HashMap::new(),
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
                            &self.textures,
                            &node.sampled_reads,
                        )?)
                    } else {
                        None
                    };

                    let target_view = if node.target == "surface" {
                        frame.surface_view.clone()
                    } else {
                        self.textures
                            .get(&node.target)
                            .ok_or_else(|| {
                                format!(
                                    "render target '{}' was not declared",
                                    node.target
                                )
                            })?
                            .view
                            .clone()
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

                    render_pass.draw(0..4, 0..1);
                }
                CompiledNode::Compute(node) => {
                    node.pass.update_if_changed(
                        device,
                        uniforms.bind_group_layout(),
                    );

                    let storage_bind_group =
                        node.pass.create_storage_bind_group(
                            device,
                            &self.textures,
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

        if self.present_source != "surface" {
            return Err(format!(
                "present source '{}' is not supported yet; use a final render node with write('surface')",
                self.present_source
            ));
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

        for name in &self.texture_resource_names {
            let needs_new = self
                .textures
                .get(name)
                .is_none_or(|texture| texture.size != [width, height]);

            if !needs_new {
                continue;
            }

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
                format: OFFSCREEN_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });

            let view =
                texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.textures.insert(
                name.clone(),
                OffscreenTexture {
                    _texture: texture,
                    view,
                    size: [width, height],
                },
            );
        }
    }
}

impl RenderPass {
    fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        node: &RenderNodeSpec,
        sampled_reads: &[String],
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let shader_path = normalize_shader_path(&node.shader_path)?;

        if !node.reads.iter().any(|resource| resource == "params") {
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
                label: Some("xtal2-texture-sampler"),
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

        let render_pipeline = create_render_pipeline(
            device,
            target_format,
            uniform_layout,
            texture_bind_group_layout.as_ref(),
            &source,
            &node.name,
        );

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
            render_pipeline,
            texture_bind_group_layout,
            sampler,
            watcher,
        })
    }

    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        textures: &HashMap<String, OffscreenTexture>,
        sampled_reads: &[String],
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

        for (index, name) in sampled_reads.iter().enumerate() {
            let texture = textures.get(name).ok_or_else(|| {
                format!("texture resource '{}' is not available", name)
            })?;

            entries.push(wgpu::BindGroupEntry {
                binding: (index + 1) as u32,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            });
        }

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("xtal2-texture-bind-group"),
            layout,
            entries: &entries,
        }))
    }

    fn update_if_changed(
        &mut self,
        device: &wgpu::Device,
        sampled_reads: &[String],
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
            uniform_layout,
            self.texture_bind_group_layout.as_ref(),
            &source,
            "xtal2-hot-reloaded",
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
        textures: &HashMap<String, OffscreenTexture>,
        target: &str,
    ) -> Result<wgpu::BindGroup, String> {
        let texture = textures.get(target).ok_or_else(|| {
            format!(
                "compute target '{}' is not a declared offscreen texture",
                target
            )
        })?;

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("xtal2-compute-storage-bind-group"),
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
            "xtal2-hot-reloaded-compute",
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
            label: Some("xtal2-pipeline-layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("xtal2-render-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
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
            topology: wgpu::PrimitiveTopology::TriangleStrip,
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
            label: Some("xtal2-compute-pipeline-layout"),
            bind_group_layouts: &[uniform_layout, storage_layout],
            push_constant_ranges: &[],
        });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("xtal2-compute-pipeline"),
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
        label: Some("xtal2-texture-bind-group-layout"),
        entries: &entries,
    })
}

fn create_storage_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("xtal2-compute-storage-bind-group-layout"),
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

fn find_present_source(graph: &GraphSpec) -> Result<String, String> {
    let mut source = None;

    for node in &graph.nodes {
        if let NodeSpec::Present { source: candidate } = node {
            if source.is_some() {
                return Err("graph can only have one Present node".to_string());
            }
            source = Some(candidate.clone());
        }
    }

    source.ok_or_else(|| "graph is missing Present node".to_string())
}

fn collect_texture_resources(resources: &[ResourceDecl]) -> Vec<String> {
    let mut names = Vec::new();

    for resource in resources {
        if matches!(resource.kind, ResourceKind::Texture2d) {
            names.push(resource.name.clone());
        }
    }

    names
}

fn validate_graph_resources(
    graph: &GraphSpec,
    texture_resource_names: &[String],
    present_source: &str,
) -> Result<(), String> {
    let texture_names = texture_resource_names
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    if present_source != "surface" && !texture_names.contains(present_source) {
        return Err(format!(
            "present source '{}' is not a declared texture resource",
            present_source
        ));
    }

    for node in &graph.nodes {
        match node {
            NodeSpec::Render(render) => {
                if render.write != "surface"
                    && !texture_names.contains(&render.write)
                {
                    return Err(format!(
                        "render node '{}' writes '{}' which is not a declared texture resource",
                        render.name, render.write
                    ));
                }

                for read in &render.reads {
                    if read == "params" {
                        continue;
                    }

                    if !texture_names.contains(read) {
                        return Err(format!(
                            "render node '{}' reads '{}' which is not a declared texture resource",
                            render.name, read
                        ));
                    }
                }
            }
            NodeSpec::Compute(compute) => {
                if !texture_names.contains(&compute.read_write) {
                    return Err(format!(
                        "compute node '{}' read_write target '{}' is not a declared texture resource",
                        compute.name, compute.read_write
                    ));
                }
            }
            NodeSpec::Present { .. } => {}
        }
    }

    Ok(())
}
