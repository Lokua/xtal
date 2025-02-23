const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec4f,

    // t1, t2, t3, t4
    a: vec4f,

    // invert, center_size, smoothness, color_mix
    b: vec4f,

    // t_long, center_y, outer_scale, center_size
    c: vec4f,

    // unused, outer_size, outer_pos_t_mix, outer_scale_2
    d: vec4f,

    // rot_angle, rot_speed, morph, unused
    e: vec4f,
    f: vec4f,
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
    // in quadrant order; t1=q1 and so on clockwise
    let t1 = params.a.x;
    let t2 = params.a.y;
    let t3 = params.a.z;
    let t4 = params.a.w;
    let invert_color = params.b.x;
    let smoothness = params.b.y;
    let blur = params.b.z;
    let color_mix = params.b.w;
    let t_long = params.c.x;
    let center_y = params.c.y;
    let outer_scale = params.c.z;
    let chord = params.d.x;
    let outer_size = 1.0 - params.d.y;
    let outer_pos_t_mix = params.d.z;
    let outer_scale_2 = params.d.w;
    let center_size = params.c.w;
    let rot_angle = params.e.x;
    let rot_speed = params.e.y;

    var p = correct_aspect(position);

    let os = mix(outer_scale, outer_scale_2, outer_pos_t_mix);
    let p1xt = mix(t4, t1, outer_pos_t_mix);
    let p1yt = mix(t3, t1, outer_pos_t_mix);
    let p2xt = mix(t2, t2, outer_pos_t_mix);
    let p2yt = mix(t1, t2, outer_pos_t_mix);
    let p3xt = mix(t4, t3, outer_pos_t_mix);
    let p3yt = mix(t3, t3, outer_pos_t_mix);
    let p4xt = mix(t4, t4, outer_pos_t_mix);
    let p4yt = mix(t3, t4, outer_pos_t_mix);
    var p1 = vec2f((1.0 - p1xt) * os, (1.0 - p1yt) * os);
    var p2 = vec2f((1.0 - p2xt) * os, (-1.0 + p2yt) * os);
    var p3 = vec2f((-1.0 + p3xt) * os, (-1.0 + p3yt) * os);
    var p4 = vec2f((-1.0 + p4xt) * os, (1.0 - p4yt) * os);
    // center
    var p5 = vec2f(0.0, center_y);

    let angle = rot_angle * TAU;
    p1 = rotate_point(p1, angle);
    p2 = rotate_point(p2, angle);
    p3 = rotate_point(p3, angle);
    p4 = rotate_point(p4, angle);
    p5 = rotate_point(p5, angle);


    let scale = 1.0;
    let d1 = length(p - p1) / scale * outer_size;
    let d2 = length(p - p2) / scale * outer_size;
    let d3 = length(p - p3) / scale * outer_size;
    let d4 = length(p - p4) / scale * outer_size;
    let d5 = length(p - p5) / (scale * 0.5);

    // As outer_size increases, smoothness decreases
    let k = smoothness * outer_size;
    let k_center = k * center_size;
    
    // Mix each corner with the center point
    let mix1 = smin(d1, d5, k_center);
    let mix2 = smin(d2, d5, k_center);
    let mix3 = smin(d3, d5, k_center);
    let mix4 = smin(d4, d5, k_center);

    // Combine all mixed pairs
    let mix12 = smin(mix1, mix2, k);
    let mix34 = smin(mix3, mix4, k);
    let final_mix = smin(mix12, mix34, k);

    let d = final_mix * blur;

    let color_1 = vec3f(
        0.5 + 0.5 * sin(p.x * 2.0),
        0.5 + 0.5 * cos(p.y * 2.0),
        0.5 + 0.5 * sin((p.x + p.y) * 1.0)
    );
    let t_long_bipolar = t_long * 2.0 - 1.0;
    let grid_resolution = 700.0 + (t_long_bipolar * 50.0);
    let color_2 = vec3f(
        0.5 + 0.5 * sin(p.x * grid_resolution),
        0.5 + 0.5 * cos(p.y * grid_resolution),
        0.5 + 0.5 * sin((p.x + p.y) * grid_resolution)
    );
    
    let base_color = mix(color_1, color_2, color_mix);
    
    // For areas where d is small (inside circles), use bright colors
    // For areas where d is large (background), fade to darker
    let circle_brightness = smoothstep(1.0, 0.9, d);
    var color = base_color * (0.3 + 0.99 * circle_brightness); 
    
    color = mix(color, 1.0 - color, invert_color);
    
    return vec4f(color, 1.0);
}

fn rotate_point(p: vec2f, angle: f32) -> vec2f {
    let rot_matrix = mat2x2f(
        cos(angle), -sin(angle),
        sin(angle), cos(angle)
    );
    return rot_matrix * p;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}