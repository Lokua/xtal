struct VertexInput {
    @location(0) position: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) pos: vec3f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // rotation, z_offset, scale, unused
    a: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    let rotation = params.a.x;
    let z_offset = clamp(params.a.y, -10.0, -0.5);
    let scale = params.a.z;

    let scaled_position = vert.position * scale;

    // Y-axis rotation
    let c = cos(rotation);
    let s = sin(rotation);
    let rotated = vec3f(
        scaled_position.x * c - scaled_position.z * s,
        scaled_position.y,
        scaled_position.x * s + scaled_position.z * c
    );
    // X-axis rotation
    // let rotated = vec3f(
    //     scaled_position.x,
    //     scaled_position.y * c - scaled_position.z * s,
    //     scaled_position.y * s + scaled_position.z * c
    // );
    // Z-axis rotation
    // let rotated = vec3f(
    //     scaled_position.x * c - scaled_position.y * s,
    //     scaled_position.x * s + scaled_position.y * c,
    //     scaled_position.z
    // );

    let translated = vec3f(rotated.x, rotated.y, rotated.z + z_offset);

    // Perspective projection matrix
    // Field of view
    let fov = radians(45.0);
    let aspect = params.resolution.x / params.resolution.y; 
    let near = 0.1;
    let far = 100.0;

    let f = 1.0 / tan(fov / 2.0);
    let range_inv = 1.0 / (near - far);

    let proj = mat4x4f(
        vec4f(f / aspect, 0.0, 0.0, 0.0),
        vec4f(0.0, f, 0.0, 0.0),
        vec4f(0.0, 0.0, far * range_inv, -1.0),
        vec4f(0.0, 0.0, near * far * range_inv, 0.0)
    );

    var out: VertexOutput;
    out.clip_position = proj * vec4f(translated, 1.0);
    out.pos = translated;
    return out;
}

@fragment
fn fs_main(@location(0) pos: vec3f) -> @location(0) vec4f {
    return vec4f(1.0, 1.0, 1.0, 1.0);
}