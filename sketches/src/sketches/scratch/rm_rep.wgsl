// https://michaelwalczyk.com/blog-ray-marching.html
// https://www.fxhash.xyz/article/unleashing-the-power-of-shaders-for-generative-art%3A-an-inside-look-at-the-creation-of-%27shoals%27

const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, map_mode
    a: vec4f,
    // space, disp_freq, rotate, twist_x
    b: vec4f,
    // warp_amt, softness, a1, a2
    c: vec4f,
    // a3, twist_y, animate_rot_x, animate_rot_y
    d: vec4f,
    // rot_x, rot_y, posterize_steps, posterize
    e: vec4f,
    // color_steps, r, g, b
    f: vec4f,
    // white_intensity, segment, segment_size, rot_t
    g: vec4f,
    // bg_noise, cam_z, segment_edge, bg_mode
    // clamp, ...
    h: vec4f,
    i: vec4f,
    j: vec4f,
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
    let t = params.a.z * 0.25;
    let posterize = bool(params.e.w);
    let cam_z = params.h.y;

    let p = correct_aspect(position);

    let cam_pos = vec3f(0.0, 0.0, cam_z);
    var ray_origin = cam_pos;
    var ray_direction = vec3f(p, 1.0);

    let color = ray_march(p, ray_origin, ray_direction);

    return select(
        vec4f(color, select(1.0, 0.01, all(color == vec3f(0.0)))), 
        vec4f(color, 1.0), 
        posterize
    );
}


fn ray_march(p: vec2f, ray_origin: vec3f, ray_direction: vec3f) -> vec3f {
    let rot_t = params.g.w;
    let t = params.a.z * rot_t;
    let rot = bool(params.b.z);
    let twist_x = params.b.w;
    let twist_y = params.d.y;
    let animate_rot_x = bool(params.d.z);
    let animate_rot_y = bool(params.d.w);
    let rot_x = params.e.x;
    let rot_y = params.e.y;
    let posterize = bool(params.e.w);
    let color_steps = params.f.x;
    let r = params.f.y;
    let g = params.f.z;
    let b = params.f.w;
    let white_intensity = params.g.x;
    let bg_noise = params.h.x;
    let bg_mode = i32(params.h.w);

    var total_distance_traveled = 0.0;
    let steps = 128;
    let min_hit_distance = 0.001;
    let max_trace_distance = 1000.0;
    let light_position = vec3f(5.0, -25.0, .0);
    let color = vec3f(r, g, b);
    let noise = fbm(p);
    let rx = select(rot_x, t, animate_rot_x);
    let ry = select(rot_y, t, animate_rot_y);

    for (var i = 0; i < steps; i++) {
        var current_p = 
            ray_origin + total_distance_traveled * ray_direction;

        if (rot) {
            let twist_angle_x = current_p.y * twist_x;
            let twist_angle_y = current_p.y * twist_y;
            current_p = rotate3(
                current_p, 
                twist_angle_y + ry, 
                twist_angle_x + rx
            );
        }

        let distance_to_closest = map(current_p);

        if (distance_to_closest < min_hit_distance) {
            let normal = calculate_normal(current_p);

            let direction_to_light = 
                normalize(current_p - light_position);

            var diffuse = max(0.02, dot(normal, direction_to_light));

            if (posterize) {
                diffuse = floor(diffuse * color_steps) / color_steps;
            }

            return vec3f(color * white_intensity) * diffuse;
        }

        if (total_distance_traveled > max_trace_distance) {
            break;
        }
        
        total_distance_traveled += distance_to_closest;
    }

    let bg_color = mix(
        color / white_intensity, 
        vec3f(noise) - color, 
        bg_noise
    );

    return swap_rgb(bg_color, bg_mode);
}

fn swap_rgb(c: vec3f, mode: i32) -> vec3f {
    // Original and single-channel fills
    if (mode == 0) { return c; }
    if (mode == 1) { return c.rrr; }
    if (mode == 2) { return c.ggg; }
    if (mode == 3) { return c.bbb; }
    
    // Double-channel duplications (one channel copied to another)
    if (mode == 4) { return c.rrg; }
    if (mode == 5) { return c.rrb; }
    if (mode == 6) { return c.rgg; }
    if (mode == 7) { return c.rbb; }
    if (mode == 8) { return c.grr; }
    if (mode == 9) { return c.brr; }
    if (mode == 10) { return c.ggr; }
    if (mode == 11) { return c.bbr; }
    
    // Channel permutations (all three channels used, just reordered)
    if (mode == 12) { return c.rbg; }
    if (mode == 13) { return c.grb; }
    if (mode == 14) { return c.gbr; }
    if (mode == 15) { return c.brg; }
    if (mode == 16) { return c.bgr; }
    
    return c;
}

fn calculate_normal(p: vec3f) -> vec3f {
    let step = vec3f(0.001, 0.0, 0.0);

    let gx = map(p + step.xyy) - map(p - step.xyy);
    let gy = map(p + step.yxy) - map(p - step.yxy);
    let gz = map(p + step.yyx) - map(p - step.yyx);

    let normal = vec3f(gx, gy, gz);

    return normalize(normal);
}

fn map(p: vec3f) -> f32 {
    let warp_amt = params.c.x;
    let softness = params.c.y;
    let map_mode = i32(params.a.w);
    let disp_freq = params.b.y;
    let posterize_steps = bool(params.e.z);
    let posterize = bool(params.e.w);
    let segment = bool(params.g.y);
    let segment_size = params.g.z;
    let segment_edge = params.h.z;
    let box_x = params.i.y;
    let box_y = params.i.z;
    let box_z = params.i.w;

    let freq = disp_freq;
    let noise = select(
        0.0, 
        fbm(p.xy) * warp_amt * 0.0025, 
        !posterize && posterize_steps
    );
    let wave = sin(freq * p);
    var product = wave.x * wave.y * wave.z;

    let segmented_value = floor(product * segment_size) / segment_size;
    let transition_factor = smoothstep(
        0.0, 
        0.06, 
        abs(fract(product * segment_size) - 0.5) - segment_edge
    );
    let smooth_segmented = mix(product, segmented_value, transition_factor);
    
    let displacement = select(
        (product + noise) * warp_amt,
        smooth_segmented * warp_amt,
        segment
    );

    let sdf1 = sdf(p, vec3f(box_x, box_y, box_z));
    let sdf2 = sdf(p, vec3f(box_x, -box_y, box_z)) + 
        displacement - 0.0618 * 1.5;

    if (map_mode == 0) {
        return sdf1 + displacement;
    }
    
    if (map_mode == 1) {
        return max(sdf1, -sdf2) + displacement;
    }

    return smax(sdf1, -sdf2, softness) + displacement;
}

// fn sdf(pos: vec3f, c: vec3f) -> f32 {
//     let space = params.b.x;
//     let clamp = params.i.x;

//     let i = round(pos / space);
//     let p = pos - space * clamp(i, vec3f(-clamp), vec3f(clamp));
//     let q = abs(p) - c;

//     return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
// }

fn sdf(pos: vec3f, c: vec3f) -> f32 {
    let space = params.b.x;
    let clamp = params.i.x;
    
    let scale = phi_scale(pos);
    let adjusted_space = space * scale;
    
    let i = round(pos / adjusted_space);
    let p = pos - adjusted_space * clamp(i, vec3f(-clamp), vec3f(clamp));
    
    let scaled_c = c * scale;
    let q = abs(p) - scaled_c;

    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn phi_scale(pos: vec3f) -> f32 {
    let dist = length(pos);
    let steps_from_center = floor(dist / 2.0);
    return pow(1.0 / PHI, steps_from_center);
}

fn fbm(p: vec2f) -> f32 {
    let OCTAVES = 5;
    let G = 0.5;

    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;

    for (var i = 0; i < OCTAVES; i++) {
        value = value + random2(p * frequency) * amplitude;
        frequency = frequency * 2.0;
        amplitude = amplitude * G;
    }

    return value;
}

fn random2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}

fn rotate3(p: vec3f, rx: f32, ry: f32) -> vec3f {
    var q = p;
    
    // Rotate around y-axis (yaw - left/right)
    let cy = cos(ry);
    let sy = sin(ry);
    q = vec3f(
        q.x * cy + q.z * sy,
        q.y,
        -q.x * sy + q.z * cy
    );
    
    // Rotate around x-axis (pitch - up/down)
    let cx = cos(rx);
    let sx = sin(rx);
    q = vec3f(
        q.x,
        q.y * cx - q.z * sx,
        q.y * sx + q.z * cx
    );
    
    return q;
}

fn smax(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(a, b, h) - k * h * (1.0 - h);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn rotate(v: vec2f, a: f32) -> vec2f {
    let s = sin(a);
    let c = cos(a);
    return vec2f(
        c * v.x - s * v.y,
        s * v.x + c * v.y,
    );
}

