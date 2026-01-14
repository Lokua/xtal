struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
    @location(2) brightness: f32
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) brightness: f32
};

struct Params {
    a: vec4f,
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vert.uv;
    out.brightness = vert.brightness;

    let rotation_x = params.a.z;
    let rotation_y = params.a.w;
    let grid_scale = params.b.x;
    let depth_mode = params.b.z;
    let depth_strength = params.b.w;

    var pos = vec3f(vert.position.xy * grid_scale, 0.0);

    if depth_mode != 0.0 {
        if depth_mode < 0.0 {
            pos.z = (1.0 - vert.brightness) * depth_strength * abs(depth_mode);
        } else {
            pos.z = vert.brightness * depth_strength * depth_mode;
        }
    }

    if rotation_x != 0.0 {
        let cx = cos(rotation_x);
        let sx = sin(rotation_x);
        pos = vec3f(
            pos.x,
            pos.y * cx - pos.z * sx,
            pos.y * sx + pos.z * cx
        );
    }

    if rotation_y != 0.0 {
        let cy = cos(rotation_y);
        let sy = sin(rotation_y);
        pos = vec3f(
            pos.x * cy - pos.z * sy,
            pos.y,
            pos.x * sy + pos.z * cy
        );
    }

    pos.z -= 2.0;

    let fov = radians(45.0);
    let aspect = params.a.x / params.a.y;
    let near = 0.1;
    let far = 100.0;
    let f = 1.0 / tan(fov / 2.0);
    let range_inv = 1.0 / (near - far);

    let proj = mat4x4<f32>(
        vec4f(f / aspect, 0.0, 0.0, 0.0),
        vec4f(0.0, f, 0.0, 0.0),
        vec4f(0.0, 0.0, far * range_inv, -1.0),
        vec4f(0.0, 0.0, near * far * range_inv, 0.0)
    );

    out.clip_position = proj * vec4f(pos, 1.0);

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let brightness_multiplier = params.b.y;
    let brightness = input.brightness * brightness_multiplier;
    return vec4f(vec3f(brightness), 1.0);
}
