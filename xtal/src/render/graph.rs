//! Declarative render graph API used by sketches.
//!
//! `GraphBuilder` lets a sketch describe render resources and pass order
//! without touching `wgpu` directly. The builder produces a `GraphSpec`, which
//! `gpu::CompiledGraph::compile` turns into concrete textures, bind groups,
//! render pipelines, compute pipelines, image uploads, and video sources.
//!
//! A graph is intentionally simple:
//!
//! - declare resources such as uniforms, offscreen textures, images, or video;
//! - add render nodes that read uniforms/textures and write to a target;
//! - add compute nodes that read/write one texture;
//! - optionally present one texture as the final output.

use std::path::PathBuf;

use crate::mesh::Mesh;

/// Stable id for the shared uniform-bank resource.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UniformHandle(usize);

/// Stable id for a texture-like graph resource.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureHandle(usize);

impl UniformHandle {
    /// Returns the numeric resource index assigned by `GraphBuilder`.
    pub fn index(self) -> usize {
        self.0
    }
}

impl TextureHandle {
    /// Returns the numeric resource index assigned by `GraphBuilder`.
    pub fn index(self) -> usize {
        self.0
    }
}

/// Type-erased graph resource handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResourceHandle {
    /// The shared uniform-bank resource.
    Uniform(UniformHandle),
    /// A texture, image, or video texture resource.
    Texture(TextureHandle),
}

/// Resource that a render pass can bind for reading.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderRead {
    /// Shared uniform banks bound at shader group 0.
    Uniform(UniformHandle),
    /// Sampled texture bound in the render pass texture group.
    Texture(TextureHandle),
}

impl From<UniformHandle> for RenderRead {
    fn from(value: UniformHandle) -> Self {
        Self::Uniform(value)
    }
}

impl From<TextureHandle> for RenderRead {
    fn from(value: TextureHandle) -> Self {
        Self::Texture(value)
    }
}

/// Destination for a render pass.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderTarget {
    /// Draw directly to the current window surface texture.
    Surface,
    /// Draw to an offscreen texture resource.
    Texture(TextureHandle),
}

/// Resource type declared in a graph recipe.
#[derive(Clone, Debug)]
pub enum ResourceKind {
    /// Shared uniform-bank buffer.
    Uniforms,
    /// Runtime-sized offscreen texture.
    Texture2d,
    /// Static image uploaded once during graph compilation.
    Image2d { path: PathBuf },
    /// Video texture updated from a named control transport source.
    Video2d { paths: Vec<PathBuf>, source: String },
}

/// Named graph resource declaration.
#[derive(Clone, Debug)]
pub struct ResourceDecl {
    /// Stable handle used by nodes.
    pub handle: ResourceHandle,
    /// Human-readable label used in generated GPU labels.
    pub name: String,
    /// Resource allocation/loading behavior.
    pub kind: ResourceKind,
}

/// Declarative render-pass node.
#[derive(Clone, Debug)]
pub struct RenderNodeSpec {
    /// Human-readable pass name.
    pub name: String,
    /// WGSL shader path containing `vs_main` and `fs_main`.
    pub shader_path: PathBuf,
    /// Meshes drawn by this pass.
    pub meshes: Vec<Mesh>,
    /// Uniform and texture resources read by this pass.
    pub reads: Vec<RenderRead>,
    /// Surface or offscreen texture written by this pass.
    pub write: RenderTarget,
}

/// Declarative compute-pass node.
#[derive(Clone, Debug)]
pub struct ComputeNodeSpec {
    /// Human-readable pass name.
    pub name: String,
    /// WGSL shader path containing `cs_main`.
    pub shader_path: PathBuf,
    /// Offscreen texture used as the compute storage target.
    pub read_write: TextureHandle,
}

/// Ordered graph operation.
#[derive(Clone, Debug)]
pub enum NodeSpec {
    /// Draw meshes through a vertex/fragment shader.
    Render(RenderNodeSpec),
    /// Dispatch a compute shader against one storage texture.
    Compute(ComputeNodeSpec),
    /// Copy a texture resource to the final surface output.
    Present { source: TextureHandle },
}

/// Complete graph recipe emitted by `GraphBuilder`.
#[derive(Clone, Debug)]
pub struct GraphSpec {
    /// Resource declarations available to graph nodes.
    pub resources: Vec<ResourceDecl>,
    /// Nodes executed in declaration order.
    pub nodes: Vec<NodeSpec>,
}

/// Builder for sketch render and compute graph recipes.
///
/// Handles returned from resource methods are lightweight ids. They become real
/// GPU resources only when the runtime compiles the final `GraphSpec`.
#[derive(Default)]
pub struct GraphBuilder {
    resources: Vec<ResourceDecl>,
    nodes: Vec<NodeSpec>,
    videos_dir: Option<PathBuf>,
    uniform_handle: Option<UniformHandle>,
    next_texture_index: usize,
    next_render_node_index: usize,
    next_compute_node_index: usize,
}

impl GraphBuilder {
    /// Creates an empty graph builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base directory used to resolve relative video paths.
    pub fn set_videos_dir(&mut self, videos_dir: impl Into<PathBuf>) {
        self.videos_dir = Some(videos_dir.into());
    }

    /// Declares or returns the shared uniform-bank resource.
    ///
    /// Xtal graphs have at most one uniform-bank resource. Calling this method
    /// repeatedly returns the same handle.
    pub fn uniforms(&mut self) -> UniformHandle {
        if let Some(handle) = self.uniform_handle {
            return handle;
        }

        let handle = UniformHandle(0);
        self.resources.push(ResourceDecl {
            handle: ResourceHandle::Uniform(handle),
            name: "params".to_string(),
            kind: ResourceKind::Uniforms,
        });
        self.uniform_handle = Some(handle);
        handle
    }

    /// Declares a runtime-sized offscreen texture resource.
    pub fn texture2d(&mut self) -> TextureHandle {
        let handle = TextureHandle(self.next_texture_index);
        self.next_texture_index += 1;

        self.resources.push(ResourceDecl {
            handle: ResourceHandle::Texture(handle),
            name: format!("tex{}", handle.0),
            kind: ResourceKind::Texture2d,
        });

        handle
    }

    /// Declares a static image texture loaded during graph compilation.
    pub fn image(&mut self, path: impl Into<PathBuf>) -> TextureHandle {
        let handle = TextureHandle(self.next_texture_index);
        self.next_texture_index += 1;

        self.resources.push(ResourceDecl {
            handle: ResourceHandle::Texture(handle),
            name: format!("img{}", handle.0),
            kind: ResourceKind::Image2d { path: path.into() },
        });

        handle
    }

    /// Declares a video texture controlled by a named video transport source.
    pub fn video(
        &mut self,
        source: impl Into<String>,
        paths: impl IntoVideoPaths,
    ) -> TextureHandle {
        let handle = TextureHandle(self.next_texture_index);
        self.next_texture_index += 1;
        let paths = paths
            .into_video_paths()
            .into_iter()
            .map(|path| self.resolve_video_path(path))
            .collect();
        let source = source.into();

        self.resources.push(ResourceDecl {
            handle: ResourceHandle::Texture(handle),
            name: source.clone(),
            kind: ResourceKind::Video2d { paths, source },
        });

        handle
    }

    /// Declares two offscreen textures for ping-pong feedback patterns.
    pub fn feedback(&mut self) -> (TextureHandle, TextureHandle) {
        (self.texture2d(), self.texture2d())
    }

    /// Starts a render node builder.
    pub fn render(&mut self) -> RenderNodeBuilder<'_> {
        let index = self.next_render_node_index;
        self.next_render_node_index += 1;

        RenderNodeBuilder {
            builder: self,
            name: format!("render_{}", index),
            shader_path: None,
            meshes: Vec::new(),
            reads: Vec::new(),
        }
    }

    /// Starts a compute node builder.
    pub fn compute(&mut self) -> ComputeNodeBuilder<'_> {
        let index = self.next_compute_node_index;
        self.next_compute_node_index += 1;

        ComputeNodeBuilder {
            builder: self,
            name: format!("compute_{}", index),
            shader_path: None,
            read_write: None,
        }
    }

    /// Adds a final presentation node that copies a texture to the surface.
    pub fn present(&mut self, source: TextureHandle) -> &mut Self {
        self.nodes.push(NodeSpec::Present { source });
        self
    }

    /// Finishes the graph recipe.
    pub fn build(self) -> GraphSpec {
        GraphSpec {
            resources: self.resources,
            nodes: self.nodes,
        }
    }

    /// Resolves relative video paths against the configured videos directory.
    fn resolve_video_path(&self, path: PathBuf) -> PathBuf {
        if path.is_absolute() {
            return path;
        }

        let Some(videos_dir) = self.videos_dir.as_ref() else {
            return path;
        };

        videos_dir.join(path)
    }
}

/// Converts common path inputs into a list of video file paths.
pub trait IntoVideoPaths {
    /// Converts the value into one or more paths for a video source.
    fn into_video_paths(self) -> Vec<PathBuf>;
}

impl IntoVideoPaths for PathBuf {
    fn into_video_paths(self) -> Vec<PathBuf> {
        vec![self]
    }
}

impl IntoVideoPaths for &str {
    fn into_video_paths(self) -> Vec<PathBuf> {
        vec![PathBuf::from(self)]
    }
}

impl IntoVideoPaths for String {
    fn into_video_paths(self) -> Vec<PathBuf> {
        vec![PathBuf::from(self)]
    }
}

impl<P> IntoVideoPaths for Vec<P>
where
    P: Into<PathBuf>,
{
    fn into_video_paths(self) -> Vec<PathBuf> {
        self.into_iter().map(Into::into).collect()
    }
}

impl<P, const N: usize> IntoVideoPaths for [P; N]
where
    P: Into<PathBuf>,
{
    fn into_video_paths(self) -> Vec<PathBuf> {
        self.into_iter().map(Into::into).collect()
    }
}

/// Builder for one render node.
pub struct RenderNodeBuilder<'a> {
    builder: &'a mut GraphBuilder,
    name: String,
    shader_path: Option<PathBuf>,
    meshes: Vec<Mesh>,
    reads: Vec<RenderRead>,
}

impl RenderNodeBuilder<'_> {
    /// Sets the WGSL shader path for this render node.
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    /// Adds a uniform or texture read to this render node.
    pub fn read(mut self, resource: impl Into<RenderRead>) -> Self {
        self.reads.push(resource.into());
        self
    }

    /// Adds a mesh draw to this render node.
    pub fn mesh(mut self, mesh: Mesh) -> Self {
        self.meshes.push(mesh);
        self
    }

    /// Finishes the render node and writes it to an offscreen texture.
    pub fn to(self, target: TextureHandle) {
        self.finish(RenderTarget::Texture(target));
    }

    /// Finishes the render node and writes it directly to the surface.
    pub fn to_surface(self) {
        self.finish(RenderTarget::Surface);
    }

    /// Validates required render node fields and appends the node.
    fn finish(self, write: RenderTarget) {
        let shader_path = self.shader_path.unwrap_or_else(|| {
            panic!("render node '{}' missing shader", self.name)
        });
        if self.meshes.is_empty() {
            panic!("render node '{}' missing mesh", self.name);
        }

        self.builder.nodes.push(NodeSpec::Render(RenderNodeSpec {
            name: self.name,
            shader_path,
            meshes: self.meshes,
            reads: self.reads,
            write,
        }));
    }
}

/// Builder for one compute node.
pub struct ComputeNodeBuilder<'a> {
    builder: &'a mut GraphBuilder,
    name: String,
    shader_path: Option<PathBuf>,
    read_write: Option<TextureHandle>,
}

impl ComputeNodeBuilder<'_> {
    /// Sets the WGSL shader path for this compute node.
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    /// Sets the texture this compute node reads and writes.
    pub fn read_write(mut self, resource: TextureHandle) -> Self {
        self.read_write = Some(resource);
        self
    }

    /// Finishes the compute node and appends it to the graph.
    pub fn dispatch(self) {
        let shader_path = self.shader_path.unwrap_or_else(|| {
            panic!("compute node '{}' missing shader", self.name)
        });

        let read_write = self.read_write.unwrap_or_else(|| {
            panic!("compute node '{}' missing read_write target", self.name)
        });

        self.builder.nodes.push(NodeSpec::Compute(ComputeNodeSpec {
            name: self.name,
            shader_path,
            read_write,
        }));
    }
}
