use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum ResourceKind {
    Uniforms,
    Texture2d,
    Image2d { path: PathBuf },
}

#[derive(Clone, Debug)]
pub struct ResourceDecl {
    pub name: String,
    pub kind: ResourceKind,
}

#[derive(Clone, Debug)]
pub struct RenderNodeSpec {
    pub name: String,
    pub shader_path: PathBuf,
    pub reads: Vec<String>,
    pub write: String,
}

#[derive(Clone, Debug)]
pub struct ComputeNodeSpec {
    pub name: String,
    pub shader_path: PathBuf,
    pub read_write: String,
}

#[derive(Clone, Debug)]
pub enum NodeSpec {
    Render(RenderNodeSpec),
    Compute(ComputeNodeSpec),
    Present { source: String },
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
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uniforms(&mut self, name: &str) -> &mut Self {
        self.resources.push(ResourceDecl {
            name: name.to_string(),
            kind: ResourceKind::Uniforms,
        });
        self
    }

    pub fn texture2d(&mut self, name: &str) -> &mut Self {
        self.resources.push(ResourceDecl {
            name: name.to_string(),
            kind: ResourceKind::Texture2d,
        });
        self
    }

    pub fn image(&mut self, name: &str, path: impl Into<PathBuf>) -> &mut Self {
        self.resources.push(ResourceDecl {
            name: name.to_string(),
            kind: ResourceKind::Image2d { path: path.into() },
        });
        self
    }

    pub fn render(&mut self, name: &str) -> RenderNodeBuilder<'_> {
        RenderNodeBuilder {
            builder: self,
            name: name.to_string(),
            shader_path: None,
            reads: Vec::new(),
            write: None,
        }
    }

    pub fn compute(&mut self, name: &str) -> ComputeNodeBuilder<'_> {
        ComputeNodeBuilder {
            builder: self,
            name: name.to_string(),
            shader_path: None,
            read_write: None,
        }
    }

    pub fn present(&mut self, source: &str) -> &mut Self {
        self.nodes.push(NodeSpec::Present {
            source: source.to_string(),
        });
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
    reads: Vec<String>,
    write: Option<String>,
}

impl RenderNodeBuilder<'_> {
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    pub fn read(mut self, resource: &str) -> Self {
        self.reads.push(resource.to_string());
        self
    }

    pub fn write(mut self, resource: &str) -> Self {
        self.write = Some(resource.to_string());
        self
    }

    pub fn add(self) {
        let shader_path = self.shader_path.unwrap_or_else(|| {
            panic!("render node '{}' missing shader", self.name)
        });

        let write = self.write.unwrap_or_else(|| {
            panic!("render node '{}' missing write target", self.name)
        });

        self.builder.nodes.push(NodeSpec::Render(RenderNodeSpec {
            name: self.name,
            shader_path,
            reads: self.reads,
            write,
        }));
    }
}

pub struct ComputeNodeBuilder<'a> {
    builder: &'a mut GraphBuilder,
    name: String,
    shader_path: Option<PathBuf>,
    read_write: Option<String>,
}

impl ComputeNodeBuilder<'_> {
    pub fn shader(mut self, shader_path: impl Into<PathBuf>) -> Self {
        self.shader_path = Some(shader_path.into());
        self
    }

    pub fn read_write(mut self, resource: &str) -> Self {
        self.read_write = Some(resource.to_string());
        self
    }

    pub fn add(self) {
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
