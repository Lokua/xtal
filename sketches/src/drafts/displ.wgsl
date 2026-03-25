struct Params {
    // w, h, beats, grid_size
    a: vec4f,

    // circle_size, pulse_rate, pulse_influence, unused
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let w = max(params.a.x, 1.0);
    let h = max(params.a.y, 1.0);
    let aspect = w / h;

    let uv = position * 0.5 + vec2f(0.5, 0.5);

    let raw_grid_size = clamp(params.a.w, 9.0, 97.0);
    let grid_size = floor(raw_grid_size * 0.5) * 2.0 + 1.0;
    let cols = grid_size;
    let rows = grid_size;
    let cell_uv = uv * vec2f(cols, rows);
    let local = fract(cell_uv) - vec2f(0.5, 0.5);
    let cell_index = clamp(
        floor(cell_uv),
        vec2f(0.0, 0.0),
        vec2f(cols - 1.0, rows - 1.0),
    );

    let center = vec2f((grid_size - 1.0) * 0.5, (grid_size - 1.0) * 0.5);
    let delta = cell_index - center;
    let dist = length(vec2f(delta.x * aspect, delta.y));
    let max_dist = max(length(vec2f(center.x * aspect, center.y)), 0.0001);

    let pulse_rate = max(params.b.y, 0.0001);
    let pulse_influence = clamp(params.b.z, 0.01, 0.5);
    let pulse_phase = fract(params.a.z / pulse_rate);
    let dist_phase = clamp(dist / max_dist, 0.0, 1.0);
    let pulse =
        exp(-pow((dist_phase - pulse_phase) / pulse_influence, 2.0));

    let base_radius = clamp(params.b.x, 0.02, 0.49);
    let radius = clamp(base_radius + 0.18 * pulse, 0.02, 0.49);
    let local_circle = vec2f(local.x * aspect, local.y);
    let circle = step(length(local_circle), radius);
    let white = vec3f(1.0);
    let red = vec3f(1.0, 0.03, 0.03);
    let color = mix(white, red, clamp(pulse, 0.0, 1.0)) * circle;
    return vec4f(color, 1.0);
}
