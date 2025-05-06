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
    // radius, disp_freq, rotate, twist_x
    b: vec4f,
    // warp_amt, softness, a1, a2
    c: vec4f,
    // a3, twist_y, animate_rot_x, animate_rot_y
    d: vec4f,
    // rot_x, rot_y, ..
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
    let t = params.a.z * 0.25;
    let mode = i32(params.a.w);
    let radius = params.b.x;
    let thickness = params.b.y;
    let step_size = params.b.z;
    let cell_size = params.b.w;
    let warp_amt = params.c.x;
    let softness = params.c.y;
    let a1 = params.c.z;
    let a2 = params.c.w;
    let a3 = params.d.x;

    let p = correct_aspect(position);

    let cam_pos = vec3f(0.0, 0.0, -2.0);
    var ray_origin = cam_pos;
    var ray_direction = vec3f(p, 1.0);

    let color = ray_march(p, ray_origin, ray_direction);

    return vec4f(color, select(1.0, 0.01, all(color == vec3f(0.0)))); 
}


fn ray_march(p: vec2f, ray_origin: vec3f, ray_direction: vec3f) -> vec3f {
    let t = params.a.z * 0.25;
    let rot = bool(params.b.z);
    let twist_x = params.b.w;
    let twist_y = params.d.y;
    let animate_rot_x = bool(params.d.z);
    let animate_rot_y = bool(params.d.w);
    let rot_x = params.e.x;
    let rot_y = params.e.y;

    var total_distance_traveled = 0.0;
    let steps = 64;
    let min_hit_distance = 0.001;
    let max_trace_distance = 1000.0;
    let bg_color = vec3f(0.45);
    let light_position = vec3f(5.0, -25.0, 2.0);
    let noise = fbm(p);
    let rx = twist_x + select(rot_x, t, animate_rot_x);
    let ry = twist_y + select(rot_y, t, animate_rot_y);

    for (var i = 0; i < steps; i++) {
        var current_position = 
            ray_origin + total_distance_traveled * ray_direction;

        if (rot) {
            let xz = rotate(current_position.xz, current_position.y * rx);
            current_position.x = xz.x;
            current_position.z = xz.y;

            let yz = rotate(current_position.yz, current_position.y *  ry);
            current_position.y = yz.x;
            current_position.z = yz.y;
        }

        let distance_to_closest = map(current_position);

        if (distance_to_closest < min_hit_distance) {
            let normal = calculate_normal(current_position);

            let direction_to_light = 
                normalize(current_position - light_position);

            let diffuse_intensity = max(0.02, dot(normal, direction_to_light));

            return vec3f(0.9) * diffuse_intensity;
        }

        if (total_distance_traveled > max_trace_distance) {
            return mix(bg_color, vec3f(noise - 0.45, noise - 0.15, noise), 0.25);
        }
        
        total_distance_traveled += distance_to_closest;
    }

    return vec3f(0.0);
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

    let freq = disp_freq;
    let noise = fbm(p.xy) * warp_amt * 0.025;
    let wave = sin(freq * p);
    let product = wave.x * wave.y * wave.z;
    let displacement = (product + noise) *  warp_amt;

    let sdf1 = distance_from_sphere(p, vec3f(0.0));
    let sdf2 = distance_from_sphere(p, vec3f(0.0)) - 0.0618;

    if (map_mode == 0) {
        return sdf1 + displacement;
    }
    
    if (map_mode == 1) {
        return max(sdf1, -sdf2) + displacement;
    }

    return smax(sdf1, -sdf2, softness) + displacement;
}

fn distance_from_sphere(p: vec3f, c: vec3f) -> f32 {
    let radius = params.b.x;
    return length(p - c) - radius;
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

