struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct Params {
    resolution: vec2f,
    mode: u32,
    radius: f32,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let aspect = params.resolution.x / params.resolution.y;
    
    // Center the coordinates (now -0.5 to 0.5)
    var centered = uv - 0.5;
    
    // Correct the x coordinate for aspect ratio
    centered.x *= aspect;
    
    let dist = length(centered);

    if (params.mode == 0u) {
        return vec4f(smoothstep(0.0, params.radius, dist));
    }
    
    return vec4f(step(dist, params.radius));
}