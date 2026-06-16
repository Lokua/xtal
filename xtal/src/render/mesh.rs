//! Mesh definitions for render graph draw calls.
//!
//! A mesh is the CPU-side vertex list that a render node uploads into a vertex
//! buffer during graph compilation. Most shader sketches use
//! `fullscreen_quad`, but custom 2D or 3D position buffers are available for
//! geometry-oriented passes.

/// Vertex layout category required by a render pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshVertexKind {
    /// Vertices contain only `vec2<f32>` positions.
    Position2D,
    /// Vertices contain only `vec3<f32>` positions.
    Position3D,
}

/// Position-only mesh data for one render node draw.
#[derive(Clone, Debug)]
pub enum Mesh {
    /// Two-dimensional positions in normalized device coordinates.
    Positions2D(Vec<[f32; 2]>),
    /// Three-dimensional positions for shaders that expect 3D vertices.
    Positions3D(Vec<[f32; 3]>),
}

impl Mesh {
    /// Returns two triangles covering the full normalized device viewport.
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

    /// Creates a 2D position mesh from caller-supplied vertices.
    pub fn positions2d(vertices: impl Into<Vec<[f32; 2]>>) -> Self {
        Self::Positions2D(vertices.into())
    }

    /// Creates a 3D position mesh from caller-supplied vertices.
    pub fn positions3d(vertices: impl Into<Vec<[f32; 3]>>) -> Self {
        Self::Positions3D(vertices.into())
    }

    /// Returns the vertex layout category needed to compile a pipeline.
    pub fn vertex_kind(&self) -> MeshVertexKind {
        match self {
            Self::Positions2D(_) => MeshVertexKind::Position2D,
            Self::Positions3D(_) => MeshVertexKind::Position3D,
        }
    }

    /// Returns the number of vertices that should be drawn.
    pub fn vertex_count(&self) -> u32 {
        match self {
            Self::Positions2D(vertices) => vertices.len() as u32,
            Self::Positions3D(vertices) => vertices.len() as u32,
        }
    }
}
