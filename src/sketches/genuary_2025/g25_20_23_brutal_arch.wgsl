struct VertexInput {
    @location(0) position: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) pos: vec3f,
};

struct Params {
    resolution: vec4f,
    // rotation, z_offset, ...unused
    a: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    let rotation = params.a.x;
    let z_offset = params.a.y;
    
    // Y-axis rotation
    let c = cos(rotation);
    let s = sin(rotation);
    let rotated = vec3f(
        vert.position.x * c - vert.position.z * s,
        vert.position.y,
        vert.position.x * s + vert.position.z * c
    );

    // Simple perspective projection
    let perspective = 1.0 / (rotated.z + z_offset);
    
    var out: VertexOutput;
    // Apply perspective to xy, keep z for depth testing
    out.clip_position = vec4f(rotated.xy * perspective, rotated.z, 1.0);
    out.pos = rotated;
    return out;
}

@fragment
fn fs_main(@location(0) pos: vec3f) -> @location(0) vec4f {
    // Color based on position for better depth visualization
    return vec4f(0.8, 0.8, 0.8, 1.0);
}