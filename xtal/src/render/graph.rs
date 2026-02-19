use std::path::PathBuf;

use crate::mesh::Mesh;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UniformHandle(usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureHandle(usize);

impl UniformHandle {
    pub fn index(self) -> usize {
        self.0
    }
}

impl TextureHandle {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResourceHandle {
    Uniform(UniformHandle),
    Texture(TextureHandle),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderRead {
    Uniform(UniformHandle),
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderTarget {
    Surface,
    Texture(TextureHandle),
}

#[derive(Clone, Debug)]
pub enum ResourceKind {
    Uniforms,
    Texture2d,
    Image2d { path: PathBuf },
}

#[derive(Clone, Debug)]
pub struct ResourceDecl {
    pub handle: ResourceHandle,
    pub name: String,
    pub kind: ResourceKind,
}

#[derive(Clone, Debug)]
pub struct RenderNodeSpec {
    pub name: String,
    pub shader_path: PathBuf,
    pub meshes: Vec<Mesh>,
    pub reads: Vec<RenderRead>,
    pub write: RenderTarget,
}

#[derive(Clone, Debug)]
pub struct ComputeNodeSpec {
    pub name: String,
    pub shader_path: PathBuf,
    pub read_write: TextureHandle,
}

#[derive(Clone, Debug)]
pub enum NodeSpec {
    Render(RenderNodeSpec),
    Compute(ComputeNodeSpec),
    Present { source: TextureHandle },
}

#[derive(Clone, Debug)]
pub struct GraphSpec {
    pub resources: Vec<ResourceDecl>,
    pub nodes: Vec<NodeSpec>,
}

#[derive(Default)]
pub struct GraphBuilder {
    resources: Vec<ResourceDecl>,
    nodes: Vec<NodeSpec>,
    uniform_handle: Option<UniformHandle>,
    next_texture_index: usize,
    next_render_node_index: usize,
    next_compute_node_index: usize,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn feedback(&mut self) -> (TextureHandle, TextureHandle) {
        (self.texture2d(), self.texture2d())
    }

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

    pub fn present(&mut self, source: TextureHandle) -> &mut Self {
        self.nodes.push(NodeSpec::Present { source });
        self
    }

    pub fn build(self) -> GraphSpec {
        GraphSpec {
            resources: self.resources,
            nodes: self.nodes,
        }
    }
}

pub struct RenderNodeBuilder<'a> {
    builder: &'a mut GraphBuilder,
    name: String,
    shader_path: Option<PathBuf>,
    meshes: Vec<Mesh>,
    reads: Vec<RenderRead>,
}

impl RenderNodeBuilder<'_> {
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    pub fn read(mut self, resource: impl Into<RenderRead>) -> Self {
        self.reads.push(resource.into());
        self
    }

    pub fn mesh(mut self, mesh: Mesh) -> Self {
        self.meshes.push(mesh);
        self
    }

    pub fn to(self, target: TextureHandle) {
        self.finish(RenderTarget::Texture(target));
    }

    pub fn to_surface(self) {
        self.finish(RenderTarget::Surface);
    }

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

pub struct ComputeNodeBuilder<'a> {
    builder: &'a mut GraphBuilder,
    name: String,
    shader_path: Option<PathBuf>,
    read_write: Option<TextureHandle>,
}

impl ComputeNodeBuilder<'_> {
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    pub fn read_write(mut self, resource: TextureHandle) -> Self {
        self.read_write = Some(resource);
        self
    }

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
