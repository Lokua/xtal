struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, depth
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let depth = params.a.w;
    let pos = correct_aspect(position);

    // Create multiple pinch points
    let num_pinches = 3.0;
    let pinch_pos = fract(pos * num_pinches);
    let pinch_center = pinch_pos - 0.5;

    // Create repeating layers going into "distance"
    let dist = length(pinch_center);
    let z = 1.0 / (dist + depth);

    // Normalize to keep zoom constant
    let base_z = 1.0 / (dist + 0.3);
    let normalized_z = z / base_z;

    let scale = normalized_z * 4.0;

    // Create box grid that scales with depth
    let p = pos * scale;
    let cell = fract(p);

    // Draw outer box edges
    let edge = 0.05;
    let box_x = step(edge, cell.x) * step(cell.x, 1.0 - edge);
    let box_y = step(edge, cell.y) * step(cell.y, 1.0 - edge);
    let outer_box = 1.0 - (box_x * box_y);

    // Draw inner box rotated 180 degrees
    // Rotate cell around center by flipping both axes
    let centered = cell - 0.5;
    let rotated = -centered + 0.5;

    // Make inner box smaller
    let inner_scale = 0.5;
    let inner_edge = edge / inner_scale;
    let inner_mask = step(0.25, cell.x) * step(cell.x, 0.75) *
        step(0.25, cell.y) * step(cell.y, 0.75);

    let inner_cell = (rotated - 0.5) / inner_scale + 0.5;
    let inner_x = step(inner_edge, inner_cell.x) *
        step(inner_cell.x, 1.0 - inner_edge);
    let inner_y = step(inner_edge, inner_cell.y) *
        step(inner_cell.y, 1.0 - inner_edge);
    let inner_box = (1.0 - (inner_x * inner_y)) * inner_mask;

    let box = max(outer_box, inner_box);

    return vec4f(vec3f(box), 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

