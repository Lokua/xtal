//! GPU compiler and executor for an xtal render graph.
//!
//! The sketch code builds a `GraphSpec` with `GraphBuilder`: "these are my
//! textures, these are my render or compute nodes, and this texture should be
//! presented." This module turns that recipe into real `wgpu` objects.
//!
//! The important split is:
//!
//! - `compile` happens when the sketch starts or switches. It reads shader
//!   files, validates WGSL, creates GPU pipelines, loads static images, and
//!   opens video sources.
//! - `execute` happens every frame. It makes sure textures have the current
//!   render size, uploads the next video frames, runs every graph node in
//!   order, and finally copies the chosen output to the window surface.
//!
//! A few wgpu terms in plain language:
//!
//! - A `Texture` is GPU image memory.
//! - A `TextureView` is the way a pass reads or writes that image.
//! - A `BindGroupLayout` says what a shader expects at each `@group` binding.
//! - A `BindGroup` is the actual set of buffers, samplers, and textures used by
//!   one draw or dispatch.
//! - A `RenderPipeline` is a compiled vertex + fragment shader setup.
//! - A `ComputePipeline` is a compiled compute shader setup.
//! - A `CommandEncoder` records GPU work. The runtime submits that work after
//!   graph execution finishes.
//!
//! Xtal's WGSL convention here is:
//!
//! - render shaders use `vs_main` and `fs_main`;
//! - compute shaders use `cs_main`;
//! - uniforms are always bind group 0;
//! - sampled textures for render passes are bind group 1;
//! - compute write targets are bind group 1 as a storage texture.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use ahash::HashMapExt;
use log::{error, info, warn};
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use wgpu::util::DeviceExt;

use crate::control::VideoTransport;
use crate::core::util::{HashMap, HashSet};
use crate::frame::Frame;
use crate::graph::{
    ComputeNodeSpec, GraphSpec, NodeSpec, RenderNodeSpec, RenderRead,
    RenderTarget, ResourceDecl, ResourceHandle, ResourceKind, TextureHandle,
};
use crate::mesh::{Mesh, MeshVertexKind};
use crate::render::video::VideoSource;
use crate::shader_watch::ShaderWatch;
use crate::uniforms::UniformBanks;

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Returns the extra bytes needed to satisfy wgpu's copy-row alignment.
///
/// GPU-to-CPU texture copies are stricter than PNG rows. Each copied row must
/// be aligned to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`, currently 256 bytes.
/// Recording and image capture use this when allocating readback buffers.
pub fn compute_row_padding(unpadded_bytes_per_row: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let rem = unpadded_bytes_per_row % align;
    if rem == 0 { 0 } else { align - rem }
}

/// GPU-ready version of a sketch graph.
///
/// `GraphSpec` is just a declarative recipe. `CompiledGraph` stores the actual
/// GPU pipelines, loaded image/video sources, live offscreen textures, and the
/// presentation rule used every frame.
pub struct CompiledGraph {
    surface_format: wgpu::TextureFormat,
    present_source: PresentSource,
    nodes: Vec<CompiledNode>,
    offscreen_resource_ids: Vec<TextureHandle>,
    offscreen_textures: HashMap<TextureHandle, GpuTexture>,
    surface_proxy_texture: Option<GpuTexture>,
    image_textures: HashMap<TextureHandle, GpuTexture>,
    video_sources: HashMap<TextureHandle, VideoSource>,
    video_source_names: HashMap<TextureHandle, String>,
    video_textures: HashMap<TextureHandle, GpuTexture>,
    texture_labels: HashMap<TextureHandle, String>,
}

/// Texture plus the small bits of metadata the executor needs later.
///
/// `texture` owns GPU memory. `view` is what render, compute, and sampling
/// passes bind. `size` lets us lazily recreate textures after resize. `format`
/// matters for recording and presentation.
struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
    format: wgpu::TextureFormat,
}

/// A graph node after its shader has been compiled for the GPU.
enum CompiledNode {
    Render(RenderNode),
    Compute(ComputeNode),
}

/// A render node with its resolved target and sampled texture dependencies.
struct RenderNode {
    name: String,
    target: RenderTarget,
    sampled_reads: Vec<TextureHandle>,
    pass: RenderPass,
}

/// A compute node that writes into one offscreen texture.
struct ComputeNode {
    name: String,
    target: TextureHandle,
    pass: ComputePass,
}

/// Where the final image comes from.
///
/// `Surface` means the graph writes directly to the window-sized render target.
/// `Texture` means a `Present` node selected an offscreen/image/video texture.
#[derive(Clone, Copy)]
enum PresentSource {
    Surface,
    Texture(TextureHandle),
}

/// Compiled state for one render pass.
///
/// A render pass draws one or more meshes through a WGSL vertex/fragment
/// shader. It always binds uniforms at group 0. If it samples textures, it also
/// owns the group 1 texture layout and sampler.
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

/// Vertex buffer prepared from a `Mesh`.
struct MeshDraw {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

/// Compiled state for one compute pass.
///
/// Xtal compute nodes write to one offscreen texture. The compute shader gets
/// uniforms at group 0 and the writable storage texture at group 1.
struct ComputePass {
    shader_path: PathBuf,
    compute_pipeline: wgpu::ComputePipeline,
    storage_bind_group_layout: wgpu::BindGroupLayout,
    watcher: Option<ShaderWatch>,
}

/// Texture resource declarations split into the groups the compiler needs.
struct TextureResources {
    offscreen: Vec<TextureHandle>,
    images: HashMap<TextureHandle, PathBuf>,
    videos: HashMap<TextureHandle, VideoResource>,
    labels: HashMap<TextureHandle, String>,
}

/// Data needed to open a video source and bind it to a control source name.
struct VideoResource {
    paths: Vec<PathBuf>,
    source: String,
}

/// Per-frame inputs supplied by the runtime when executing the graph.
///
/// Most fields are borrowed from `RuntimeContext`, `ControlHub`, and frame
/// timing.
/// This keeps `CompiledGraph` focused on GPU work while the runtime remains the
/// owner of application state.
pub struct ExecuteCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub uniforms: &'a UniformBanks,
    pub video_transports: &'a HashMap<String, VideoTransport>,
    pub beats: f32,
    pub bpm: f32,
    pub render_size: [u32; 2],
    pub surface_size: [u32; 2],
}

impl CompiledGraph {
    /// Turns a graph recipe into GPU objects.
    ///
    /// This is intentionally front-loaded work. Shader files are read and
    /// compiled, render/compute pipelines are built, image files are uploaded,
    /// and video sources are opened here so the per-frame path can stay small.
    ///
    /// Offscreen textures are not allocated here because their size depends on
    /// the current render size. They are created lazily in `execute`.
    pub fn compile(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        graph: GraphSpec,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        // A graph may either render directly to the surface or present one
        // texture explicitly. The latter is what enables multi-pass graphs:
        // pass A writes a texture, pass B reads it, Present shows the result.
        let present_source_handle = find_present_source(&graph)?;
        let TextureResources {
            offscreen: offscreen_resource_ids,
            images: image_resources,
            videos: video_resources,
            labels: texture_labels,
        } = collect_texture_resources(&graph.resources);

        // Validate references before creating GPU objects. Without this, a
        // missing texture handle would fail later in the middle of rendering.
        validate_graph_resources(
            &graph,
            &offscreen_resource_ids,
            &image_resources,
            &video_resources,
            present_source_handle,
        )?;

        let mut nodes = Vec::new();

        // Compile executable nodes in graph order. The order matters: a later
        // render pass can sample a texture written by an earlier pass.
        for node in graph.nodes {
            match node {
                NodeSpec::Render(render) => {
                    // Uniform reads use the shared uniform bind group. Texture
                    // reads need per-pass bind group layouts and views.
                    let sampled_reads = render
                        .reads
                        .iter()
                        .filter_map(|resource| match resource {
                            RenderRead::Texture(texture) => Some(*texture),
                            RenderRead::Uniform(_) => None,
                        })
                        .collect::<Vec<_>>();

                    // A shader pipeline must know the format it writes to.
                    // Window surfaces use the platform-selected format;
                    // offscreen textures use Xtal's fixed internal format.
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

        // Images are static, so load and upload them once at compile time.
        let mut image_textures = HashMap::new();

        for (handle, path) in image_resources {
            let label = texture_labels
                .get(&handle)
                .map(|name| name.as_str())
                .unwrap_or("xtal-image-texture");
            let texture = load_image_texture(device, queue, label, &path)?;
            image_textures.insert(handle, texture);
        }

        // Videos are dynamic. Open the source now, but start with a 1x1
        // placeholder texture until the first decoded frame arrives.
        let mut video_sources = HashMap::new();
        let mut video_source_names = HashMap::new();
        let mut video_textures = HashMap::new();

        for (handle, resource) in video_resources {
            let source = VideoSource::new(&resource.paths)?;
            video_sources.insert(handle, source);
            video_source_names.insert(handle, resource.source);
            let label = texture_labels
                .get(&handle)
                .map(|name| name.as_str())
                .unwrap_or("xtal-video-texture");
            video_textures.insert(
                handle,
                create_placeholder_texture(device, queue, label),
            );
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
            surface_proxy_texture: None,
            image_textures,
            video_sources,
            video_source_names,
            video_textures,
            texture_labels,
        })
    }

    /// Executes the compiled graph for one frame.
    ///
    /// The runtime has already updated the control hub and uploaded uniforms
    /// before calling this. This method only records GPU commands into the
    /// frame encoder; the runtime submits those commands after recording and
    /// capture copies have also been encoded.
    pub fn execute(
        &mut self,
        frame: &mut Frame,
        ctx: ExecuteCtx<'_>,
    ) -> Result<(), String> {
        // Resize-sensitive resources are maintained lazily. This avoids
        // rebuilding pipelines just because the window or projector size
        // changed.
        self.ensure_offscreen_textures(ctx.device, ctx.render_size);
        self.ensure_surface_proxy_texture(
            ctx.device,
            ctx.render_size,
            ctx.surface_size,
        );
        // Video sources are advanced from beat/time controls before any render
        // pass samples them.
        self.update_video_textures(
            ctx.device,
            ctx.queue,
            ctx.video_transports,
            ctx.beats,
            ctx.bpm,
        )?;

        // The graph builder order is the execution order. Xtal does not infer a
        // dependency graph here; sketch setup must add nodes in the order they
        // should run.
        for node in &mut self.nodes {
            match node {
                CompiledNode::Render(node) => {
                    // Hot-reload swaps the pipeline in place if the WGSL file
                    // changed. Bad reloads keep the previous working pipeline.
                    node.pass.update_if_changed(
                        ctx.device,
                        &node.sampled_reads,
                        ctx.uniforms.bind_group_layout(),
                    );

                    // Texture bind groups are created per frame because the
                    // texture views can change after resize or video decode.
                    let texture_bind_group = if !node.sampled_reads.is_empty() {
                        Some(node.pass.create_texture_bind_group(
                            ctx.device,
                            &self.offscreen_textures,
                            &self.image_textures,
                            &self.video_textures,
                            &node.sampled_reads,
                        )?)
                    } else {
                        None
                    };

                    // Render passes write either to the window path or to an
                    // offscreen texture that another node can sample later.
                    let target_view = match node.target {
                        RenderTarget::Surface => self
                            .surface_proxy_texture
                            .as_ref()
                            .map(|texture| texture.view.clone())
                            .unwrap_or_else(|| frame.surface_view.clone()),
                        RenderTarget::Texture(texture) => self
                            .offscreen_textures
                            .get(&texture)
                            .ok_or_else(|| {
                                format!(
                                    "render target '{}' was not declared as \
                                        texture2d",
                                    texture_label(
                                        texture,
                                        &self.texture_labels
                                    )
                                )
                            })?
                            .view
                            .clone(),
                    };

                    // Beginning a render pass records "draw into this target"
                    // commands into the frame command encoder.
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
                    render_pass.set_bind_group(
                        0,
                        ctx.uniforms.bind_group(),
                        &[],
                    );

                    if let Some(bind_group) = texture_bind_group.as_ref() {
                        render_pass.set_bind_group(1, bind_group, &[]);
                    }

                    // Each mesh is drawn with the same shader and bindings.
                    for mesh in &node.pass.meshes {
                        render_pass
                            .set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        render_pass.draw(0..mesh.vertex_count, 0..1);
                    }
                }
                CompiledNode::Compute(node) => {
                    // Compute shaders do not draw triangles. They dispatch
                    // workgroups that write pixels into a storage texture.
                    node.pass.update_if_changed(
                        ctx.device,
                        ctx.uniforms.bind_group_layout(),
                    );

                    let storage_bind_group =
                        node.pass.create_storage_bind_group(
                            ctx.device,
                            &self.offscreen_textures,
                            &node.target,
                        )?;

                    // Xtal compute shaders are dispatched in 8x8 workgroups.
                    // Shader code should guard against invocation ids outside
                    // the actual image bounds on non-multiple-of-8 sizes.
                    let width = ctx.render_size[0].max(1);
                    let height = ctx.render_size[1].max(1);
                    let workgroup_x = width.div_ceil(8);
                    let workgroup_y = height.div_ceil(8);

                    let mut compute_pass = frame.encoder().begin_compute_pass(
                        &wgpu::ComputePassDescriptor {
                            label: Some(&node.name),
                            timestamp_writes: None,
                        },
                    );

                    compute_pass.set_pipeline(&node.pass.compute_pipeline);
                    compute_pass.set_bind_group(
                        0,
                        ctx.uniforms.bind_group(),
                        &[],
                    );
                    compute_pass.set_bind_group(1, &storage_bind_group, &[]);
                    compute_pass.dispatch_workgroups(
                        workgroup_x,
                        workgroup_y,
                        1,
                    );
                }
            }
        }

        // Presentation is the final copy to the actual surface. Even when a
        // graph wrote to an intermediate texture, the window only displays the
        // surface texture acquired by the runtime for this frame.
        if let PresentSource::Texture(source) = self.present_source {
            let source_view =
                if let Some(texture) = self.offscreen_textures.get(&source) {
                    texture.view.clone()
                } else if let Some(texture) = self.image_textures.get(&source) {
                    texture.view.clone()
                } else if let Some(texture) = self.video_textures.get(&source) {
                    texture.view.clone()
                } else {
                    return Err(format!(
                        "present source '{}' is not a known texture resource",
                        texture_label(source, &self.texture_labels)
                    ));
                };

            blit_texture_to_surface(
                ctx.device,
                frame,
                &source_view,
                self.surface_format,
            );
        } else if let Some(texture) = self.surface_proxy_texture.as_ref() {
            blit_texture_to_surface(
                ctx.device,
                frame,
                &texture.view,
                self.surface_format,
            );
        }

        Ok(())
    }

    /// Resets video sources and clears their current GPU textures.
    ///
    /// This runs on transport reset, MIDI Start/Continue, and sketch reset so
    /// beat-synced video returns to the start with the rest of the graph.
    pub fn reset(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        video_transports: &HashMap<String, VideoTransport>,
        bpm: f32,
    ) {
        for (handle, source) in &mut self.video_sources {
            let result = if let Some(transport) = self
                .video_source_names
                .get(handle)
                .and_then(|name| video_transports.get(name))
            {
                source.restart_with_transport(transport, bpm)
            } else {
                source.restart()
            };

            if let Err(err) = result {
                warn!(
                    "failed to restart video '{}': {}",
                    texture_label(*handle, &self.texture_labels),
                    err
                );
            }

            let label = texture_label(*handle, &self.texture_labels);
            self.video_textures.insert(
                *handle,
                create_placeholder_texture(device, queue, label),
            );
        }
    }

    /// Updates every video-backed GPU texture for the current frame.
    ///
    /// If a video has a named transport in the control hub, that transport is
    /// applied before decoding. The decoded RGBA frame is then uploaded into
    /// the texture that render passes sample.
    fn update_video_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        video_transports: &HashMap<String, VideoTransport>,
        beats: f32,
        bpm: f32,
    ) -> Result<(), String> {
        for (handle, source) in &mut self.video_sources {
            if let Some(source_name) = self.video_source_names.get(handle)
                && let Some(transport) = video_transports.get(source_name)
            {
                source.apply_transport(transport, beats, bpm)?;
            }

            let Some(frame) = source.next_frame()? else {
                continue;
            };

            let width = frame.width.max(1);
            let height = frame.height.max(1);
            let needs_new = self
                .video_textures
                .get(handle)
                .is_none_or(|texture| texture.size != [width, height]);

            if needs_new {
                let label = texture_label(*handle, &self.texture_labels);
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
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
                let view = texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                self.video_textures.insert(
                    *handle,
                    GpuTexture {
                        texture,
                        view,
                        size: [width, height],
                        format: IMAGE_FORMAT,
                    },
                );
            }

            let texture = self.video_textures.get(handle).ok_or_else(|| {
                format!(
                    "video texture '{}' was not created",
                    texture_label(*handle, &self.texture_labels)
                )
            })?;

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &frame.rgba,
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
        }

        Ok(())
    }

    /// Ensures every declared offscreen texture exists at the render size.
    ///
    /// These textures are where multi-pass render nodes and compute nodes
    /// write.
    /// They are recreated on resize, which invalidates old `TextureView`s and
    /// is why render bind groups are rebuilt each frame.
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

    /// Creates the proxy used when render size differs from surface size.
    ///
    /// In projector mode the graph can render internally at a lower resolution
    /// than the actual window. In that case direct-to-surface render nodes draw
    /// into this proxy first, and the proxy is scaled to the real surface at
    /// the end of `execute`.
    fn ensure_surface_proxy_texture(
        &mut self,
        device: &wgpu::Device,
        render_size: [u32; 2],
        surface_size: [u32; 2],
    ) {
        if !matches!(self.present_source, PresentSource::Surface)
            || render_size == surface_size
        {
            self.surface_proxy_texture = None;
            return;
        }

        let width = render_size[0].max(1);
        let height = render_size[1].max(1);
        let needs_new = self
            .surface_proxy_texture
            .as_ref()
            .is_none_or(|texture| texture.size != [width, height]);

        if !needs_new {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("xtal-projector-surface-proxy"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.surface_proxy_texture = Some(GpuTexture {
            texture,
            view,
            size: [width, height],
            format: self.surface_format,
        });
    }

    /// Returns the texture that recording should copy from.
    ///
    /// If the graph presents an offscreen/image/video texture, that exact
    /// texture is the recording source. If the graph presents the surface path,
    /// recording only has a texture source when a surface proxy exists.
    pub fn recording_source_texture(&self) -> Option<&wgpu::Texture> {
        match self.present_source {
            PresentSource::Surface => self
                .surface_proxy_texture
                .as_ref()
                .map(|texture| &texture.texture),
            PresentSource::Texture(source) => self
                .offscreen_textures
                .get(&source)
                .map(|texture| &texture.texture)
                .or_else(|| {
                    self.image_textures
                        .get(&source)
                        .map(|texture| &texture.texture)
                })
                .or_else(|| {
                    self.video_textures
                        .get(&source)
                        .map(|texture| &texture.texture)
                }),
        }
    }

    /// Returns the format of `recording_source_texture`.
    pub fn recording_source_format(&self) -> Option<wgpu::TextureFormat> {
        match self.present_source {
            PresentSource::Surface => self
                .surface_proxy_texture
                .as_ref()
                .map(|texture| texture.format),
            PresentSource::Texture(source) => self
                .offscreen_textures
                .get(&source)
                .map(|texture| texture.format)
                .or_else(|| {
                    self.image_textures
                        .get(&source)
                        .map(|texture| texture.format)
                })
                .or_else(|| {
                    self.video_textures
                        .get(&source)
                        .map(|texture| texture.format)
                }),
        }
    }
}

impl RenderPass {
    /// Builds one render pipeline from a render node spec.
    ///
    /// The shader must read `params` because Xtal always exposes runtime
    /// controls through the uniform bank. Texture reads are optional; when a
    /// node has them, this creates the sampler and texture bind group layout
    /// expected by the shader.
    fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        node: &RenderNodeSpec,
        sampled_reads: &[TextureHandle],
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let shader_path = normalize_shader_path(&node.shader_path)?;

        // This keeps shader/runtime expectations explicit. If a render node
        // does not declare the uniform read, it would not get bind group 0.
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

        // Binding layout for sampled textures:
        // - binding 0 is one shared sampler;
        // - bindings 1..N are the texture views in node.read order.
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

    /// Creates the actual texture bind group for this frame.
    ///
    /// The layout was created at compile time, but the views can change at
    /// runtime because offscreen textures resize and video textures can change
    /// dimensions. Rebuilding this small bind group per frame keeps it correct.
    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        offscreen_textures: &HashMap<TextureHandle, GpuTexture>,
        image_textures: &HashMap<TextureHandle, GpuTexture>,
        video_textures: &HashMap<TextureHandle, GpuTexture>,
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
            } else if let Some(texture) = video_textures.get(handle) {
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

    /// Reloads this pass's WGSL file if the watcher reports a change.
    ///
    /// Failed reloads are logged and ignored, leaving the last valid pipeline
    /// in place so live coding mistakes do not kill the running sketch.
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
                "shader '{}' reads textures but no texture bind group layout \
                    is configured",
                self.shader_path.display()
            );
        }

        info!("shader reload applied: {}", self.shader_path.display());
    }
}

impl ComputePass {
    /// Builds one compute pipeline from a compute node spec.
    ///
    /// A compute node has exactly one writable texture target. That texture is
    /// bound as a write-only storage texture at group 1 binding 0.
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

    /// Creates the storage bind group for this frame's compute target.
    ///
    /// Like render texture bind groups, this is rebuilt as needed because the
    /// target texture can be recreated after a resize.
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

    /// Reloads this compute shader if the WGSL file changed.
    ///
    /// A failed reload keeps the previous working compute pipeline alive.
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

/// Creates a render pipeline for Xtal's render shader convention.
///
/// The shader must expose `vs_main` and `fs_main`. Bind group 0 is uniforms;
/// bind group 1 exists only when the render node samples textures.
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// Returns the vertex-buffer layout matching the mesh data for a render node.
fn vertex_buffer_layout_for_kind(
    mesh_kind: MeshVertexKind,
) -> wgpu::VertexBufferLayout<'static> {
    match mesh_kind {
        MeshVertexKind::Position2D => wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>()
                as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_2D_VERTEX_ATTRIBUTES,
        },
        MeshVertexKind::Position3D => wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>()
                as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_3D_VERTEX_ATTRIBUTES,
        },
    }
}

/// Uploads a CPU mesh into a GPU vertex buffer.
fn create_mesh_draw(device: &wgpu::Device, mesh: &Mesh) -> MeshDraw {
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

/// Ensures every mesh in one render node uses the same vertex format.
///
/// A single wgpu render pipeline has one vertex-buffer layout. Mixing 2D and 3D
/// meshes in the same node would require different layouts, so Xtal rejects it
/// early.
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
                "render node '{}' has mixed mesh vertex kinds; mesh {} \
                    differs from first mesh",
                node.name, index
            ));
        }
    }

    Ok(mesh_kind)
}

/// Creates a compute pipeline for Xtal's compute shader convention.
///
/// The shader must expose `cs_main`. Bind group 0 is uniforms and bind group 1
/// is the writable storage texture.
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

/// Creates the group 1 layout used by render passes that sample textures.
///
/// The corresponding WGSL shape is:
///
/// ```wgsl
/// @group(1) @binding(0) var samp: sampler;
/// @group(1) @binding(1) var tex0: texture_2d<f32>;
/// @group(1) @binding(2) var tex1: texture_2d<f32>;
/// ```
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

/// Creates the group 1 layout used by compute passes.
///
/// The compute shader writes pixels into this storage texture. The texture
/// format must match `OFFSCREEN_FORMAT`, because compute targets are always
/// declared offscreen `texture2d` resources.
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

/// Draws one texture onto the real window surface.
///
/// This is the final presentation copy. It uses a tiny built-in shader rather
/// than asking every sketch shader to know about surface scaling and formats.
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

/// Loads a PNG file and uploads it into a sampled GPU texture.
///
/// Xtal currently accepts 8-bit RGB and RGBA PNGs here. RGB data is expanded to
/// RGBA because the GPU upload path and shaders expect four bytes per pixel.
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
                "unsupported PNG format for '{}': {:?} {:?} \
                    (expected RGB/RGBA 8-bit)",
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

/// Creates a black 1x1 texture used until real image/video data is available.
///
/// This prevents render passes from failing just because a video source has not
/// produced its first frame yet.
fn create_placeholder_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    name: &str,
) -> GpuTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(name),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
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
        &[0, 0, 0, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    GpuTexture {
        texture,
        view,
        size: [1, 1],
        format: IMAGE_FORMAT,
    }
}

/// Parses and validates WGSL before creating a wgpu pipeline.
///
/// This catches many shader errors at graph compile or hot-reload time and
/// lets Xtal print a normal error instead of failing later in GPU submission.
fn validate_shader(source: &str) -> Result<(), String> {
    let module = wgsl::parse_str(source).map_err(|err| err.to_string())?;

    let mut validator =
        Validator::new(ValidationFlags::all(), Capabilities::all());

    validator
        .validate(&module)
        .map_err(|err| err.to_string())
        .map(|_| ())
}

/// Resolves relative asset paths against the current process directory.
///
/// Sketches usually pass paths relative to the workspace they are run from.
/// This helper turns them into absolute paths before shader/image loading and
/// file watching.
fn normalize_shader_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to get current directory: {}", err))?;

    Ok(cwd.join(path))
}

/// Finds the single optional `Present` node in a graph.
///
/// Without a `Present` node, direct-to-surface rendering is used. With one,
/// the selected texture is copied to the surface after all executable nodes
/// run.
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

/// Splits graph resource declarations into concrete texture categories.
///
/// The graph builder stores all resources together. The compiler needs separate
/// maps because offscreen textures, images, and videos are created and updated
/// differently.
fn collect_texture_resources(resources: &[ResourceDecl]) -> TextureResources {
    let mut offscreen = Vec::new();
    let mut images = HashMap::new();
    let mut videos = HashMap::new();
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
            ResourceKind::Video2d { paths, source } => {
                videos.insert(
                    handle,
                    VideoResource {
                        paths: paths.clone(),
                        source: source.clone(),
                    },
                );
            }
            ResourceKind::Uniforms => unreachable!(),
        }
    }

    TextureResources {
        offscreen,
        images,
        videos,
        labels,
    }
}

/// Checks that graph nodes only refer to declared texture resources.
///
/// This is a friendly validation pass before GPU execution. It catches common
/// mistakes like rendering to an undeclared texture, sampling a missing
/// texture, or asking a compute node to write to an image/video texture.
fn validate_graph_resources(
    graph: &GraphSpec,
    offscreen_resource_ids: &[TextureHandle],
    image_resources: &HashMap<TextureHandle, PathBuf>,
    video_resources: &HashMap<TextureHandle, VideoResource>,
    present_source: Option<TextureHandle>,
) -> Result<(), String> {
    let offscreen_ids = offscreen_resource_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let image_ids = image_resources.keys().copied().collect::<HashSet<_>>();
    let video_ids = video_resources.keys().copied().collect::<HashSet<_>>();

    if let Some(source) = present_source
        && !offscreen_ids.contains(&source)
        && !image_ids.contains(&source)
        && !video_ids.contains(&source)
    {
        return Err(format!(
            "present source texture {} is not a declared \
                offscreen/image/video texture resource",
            source.index()
        ));
    }

    for node in &graph.nodes {
        match node {
            NodeSpec::Render(render) => {
                if let RenderTarget::Texture(target) = render.write
                    && !offscreen_ids.contains(&target)
                {
                    return Err(format!(
                        "render node '{}' writes texture {} which is not a \
                            declared texture2d resource",
                        render.name,
                        target.index()
                    ));
                }

                for read in &render.reads {
                    if let RenderRead::Texture(texture) = read
                        && !offscreen_ids.contains(texture)
                        && !image_ids.contains(texture)
                        && !video_ids.contains(texture)
                    {
                        return Err(format!(
                            "render node '{}' reads texture {} which is not a \
                                declared texture2d/image/video resource",
                            render.name,
                            texture.index()
                        ));
                    }
                }
            }
            NodeSpec::Compute(compute) => {
                if !offscreen_ids.contains(&compute.read_write) {
                    return Err(format!(
                        "compute node '{}' read_write target '{}' is not a \
                            declared texture2d resource",
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

/// Returns the human-readable graph label for diagnostics.
fn texture_label(
    handle: TextureHandle,
    labels: &HashMap<TextureHandle, String>,
) -> &str {
    labels.get(&handle).map(String::as_str).unwrap_or("texture")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projector_stress_shader_is_valid_wgsl() {
        validate_shader(include_str!(
            "../../../sketches/src/dev/projector_stress.wgsl"
        ))
        .expect("projector stress shader should parse and validate");
    }
}
