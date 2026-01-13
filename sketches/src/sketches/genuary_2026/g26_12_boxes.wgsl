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
    // boxify, cell_size, num_pinches, unused
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
    let quant = params.b.x;
    let cell_size = params.b.y;
    let num_pinches = params.b.z;
    let pos = correct_aspect(position);

    // Create multiple pinch points
    let pinch_pos = fract(pos * num_pinches);
    let pinch_center = pinch_pos - 0.5;

    // Create repeating layers going into "distance"
    let dist = length(pinch_center);
    let z = 1.0 / (dist + depth);

    // Normalize to keep zoom constant
    let base_z = 1.0 / (dist + 0.3);
    let normalized_z = z / base_z;

    // Quantize the scale to create stepped/blocky distortion
    let quant_steps = mix(100.0, 5.0, quant);
    let quantized_z = floor(normalized_z * quant_steps) / quant_steps;
    let final_z = mix(normalized_z, quantized_z, quant);

    let scale = final_z * 3.0;

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
    let inner_mask = step(cell_size, cell.x) *
        step(cell.x, 1.0 - cell_size) *
        step(cell_size, cell.y) *
        step(cell.y, 1.0 - cell_size);

    let inner_cell = (rotated - 0.5) / inner_scale + 0.5;
    let inner_x = step(inner_edge, inner_cell.x) *
        step(inner_cell.x, 1.0 - inner_edge);
    let inner_y = step(inner_edge, inner_cell.y) *
        step(inner_cell.y, 1.0 - inner_edge);
    let inner_box = (1.0 - (inner_x * inner_y)) * inner_mask;

    // Draw connecting lines between inner and outer corners
    let line_width = 0.02;

    // Distance to diagonal lines connecting corners
    let to_corner = abs(cell.x - cell.y);
    let to_anti_corner = abs(cell.x - (1.0 - cell.y));

    // Only draw lines in regions between inner and outer boxes
    let in_corner_region = (cell.x < cell_size || cell.x > 1.0 - cell_size) ||
        (cell.y < cell_size || cell.y > 1.0 - cell_size);

    let diagonal1 = step(to_corner, line_width) *
        f32(in_corner_region);
    let diagonal2 = step(to_anti_corner, line_width) *
        f32(in_corner_region);
    let connecting_lines = max(diagonal1, diagonal2);

    let box = max(max(outer_box, inner_box), connecting_lines);

    // Vary brightness based on distance from pinch center
    // Closer to pinch = brighter
    let min_brightness = 0.1;
    let falloff = 2.0;
    let brightness = mix(1.0, min_brightness, clamp(dist * falloff, 0.0, 1.0));


    let coral = vec3f(1.0, 0.5, 0.35);
    let color = vec3f(box) * coral * brightness;

    return vec4f(color, 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

