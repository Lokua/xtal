#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshVertexKind {
    Position2D,
    Position3D,
}

#[derive(Clone, Debug)]
pub enum Mesh {
    Positions2D(Vec<[f32; 2]>),
    Positions3D(Vec<[f32; 3]>),
}

impl Mesh {
    pub fn fullscreen_quad() -> Self {
        Self::Positions2D(vec![
            [-1.0, -1.0],
            [1.0, -1.0],
            [-1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [1.0, 1.0],
        ])
    }

    pub fn positions2d(vertices: impl Into<Vec<[f32; 2]>>) -> Self {
        Self::Positions2D(vertices.into())
    }

    pub fn positions3d(vertices: impl Into<Vec<[f32; 3]>>) -> Self {
        Self::Positions3D(vertices.into())
    }

    pub fn vertex_kind(&self) -> MeshVertexKind {
        match self {
            Self::Positions2D(_) => MeshVertexKind::Position2D,
            Self::Positions3D(_) => MeshVertexKind::Position3D,
        }
    }

    pub fn vertex_count(&self) -> u32 {
        match self {
            Self::Positions2D(vertices) => vertices.len() as u32,
            Self::Positions3D(vertices) => vertices.len() as u32,
        }
    }
}
