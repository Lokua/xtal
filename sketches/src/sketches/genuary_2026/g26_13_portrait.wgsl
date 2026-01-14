struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
    @location(2) color: vec3f
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec3f
};

struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

fn hash(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vert.uv;
    out.color = vert.color;

    let grid_scale = params.b.x;
    let depth_mode_slider = params.b.y;
    let depth_strength = params.b.z;

    let animate_x = params.c.x;
    let animate_y = params.c.y;
    let rotation_x_anim = params.c.z;
    let rotation_y_anim = params.c.w;
    let rotation_x_slider = params.a.z;
    let rotation_y_slider = params.a.w;

    let animate_depth = params.d.x;
    let shape_chaos = params.d.y;
    let depth_mode_anim = params.d.z;

    let rotation_x = select(rotation_x_slider, rotation_x_anim, animate_x > 0.5);
    let rotation_y = select(rotation_y_slider, rotation_y_anim, animate_y > 0.5);
    let depth_mode = select(depth_mode_slider, depth_mode_anim, animate_depth > 0.5);

    var pos = vec3f(vert.position.xy * grid_scale, 0.0);

    let brightness = (vert.color.r + vert.color.g + vert.color.b) / 3.0;

    if shape_chaos > 0.0 {
        let cell_id = floor(vert.uv * 256.0);
        let random_val = hash(cell_id);
        let angle = random_val * 6.28318;
        let offset_scale = (random_val * 2.0 - 1.0) * 0.015;

        let local_uv = fract(vert.uv * 256.0);
        let centered = (local_uv - 0.5) * 2.0;

        let rotated_x = centered.x * cos(angle * shape_chaos) - centered.y * sin(angle * shape_chaos);
        let rotated_y = centered.x * sin(angle * shape_chaos) + centered.y * cos(angle * shape_chaos);

        pos.x += (rotated_x - centered.x) * 0.003 * shape_chaos;
        pos.y += (rotated_y - centered.y) * 0.003 * shape_chaos;
        pos.x += offset_scale * shape_chaos;
        pos.y += offset_scale * random_val * shape_chaos;
    }

    if depth_mode != 0.0 {
        if depth_mode < 0.0 {
            pos.z = (1.0 - brightness) * depth_strength * abs(depth_mode);
        } else {
            pos.z = brightness * depth_strength * depth_mode;
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
    let lavender = vec3f(0.75, 0.58, 0.89);
    let brightness = (input.color.r + input.color.g + input.color.b) / 3.0;
    let out_color = lavender * brightness;
    return vec4f(out_color, 1.0);
}
