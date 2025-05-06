const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;

// Modes
const GRADIENT: i32 = 0;
const GRADIENT_STEPPED: i32 = 1;
const CIRCLE: i32 = 2;
const CIRCLE_STEPPED: i32 = 3;
const GRID: i32 = 4;
const GRID_SMOOTH: i32 = 5;
const GRID_RADIAL: i32 = 6;
const GRID_WARPED: i32 = 7;
const RAY_MARCH: i32 = 8;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, mode
    a: vec4f,
    // radius, thickness, step_size, cell_size
    b: vec4f,
    // warp_amt, softness, a1, a2
    c: vec4f,
    // a3, ...
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

    if (mode == GRADIENT) {
        let d = length(p);
        return vec4f(vec3f(d), 1.0);
    } 
    
    if (mode == GRADIENT_STEPPED) {
        let o = fract(length(p) * step_size);
        return vec4f(vec3f(o), 1.0);
    } 

    if (mode == CIRCLE) {
        let d = length(p) - radius;
        let o = select(0.0, 1.0, d < radius);
        return vec4f(vec3f(o), 1.0); 
    }
    
    if (mode == CIRCLE_STEPPED) {
        let d = length(p) - radius;
        let o = fract(d * step_size);
        let masked = select(0.0, o, d < 0.0);
        return vec4f(vec3f(masked), 1.0);
    }

    if (mode == GRID) {
        // # Example
        // p = vec2f(0.3, 0.6); cell_size = 0.25 
        // p / cell_size == vec2f(0.3 / 0.25, 0.6 / 0.25) == vec2f(1.2, 2.4)
        // == 1.2 cells to the right; 2.4 cells up from the origin 
        // The integer part is which cell, the fractional part is where inside
        // - floor(cp) = what cell
        // - fract(cp) = where inside
        let cp = p / cell_size;

        // cell coordinates:

        //        y ↑
        //          │
        //     1.0  ┼──────┬──────┐
        //          │      │      │
        //          │      │      │
        //     0.5  ┼──────●──────┤  ◄── center = (0.5, 0.5)
        //          │      │      │
        //          │      │      │
        //     0.0  └──────┴──────┘
        //          0.0   0.5   1.0 → x

        // (0.0, 0.0) is bottom left
        // (0.5, 0.5) is center
        // (1.0, 1.0) bottom left of next cell

        // fract(x) == 0.2 == 20% across the cell (from left to right) 
        // fract(y) == 0.4 == 40% up the cell (from bottom to top)
        //
        // -0.5 remaps coordinate system from [0, 1] to [-0.5, 0.5] so the
        // origin is at 0.0 
        // 
        // fract(x) - 0.5 == 0.2 - 0.5 = -0.3 == 30% left of center
        // fract(y) - 0.5 == 0.4 - 0.5 = -0.1 == 10% below center
        //
        // abs == distance from center
        // 0.0 = center, 0.5 means edge
        let d = abs(fract(cp) - 0.5);

        // step(edge, x) returns 0.0 if x < edge, else 1.0
        // step(edge, x) == x < edge ? 0.0 : 1.0
        // Perhaps easier to think of "edge" as "threshold"
        // or.."returns 1 when x has stepped over the edge"
        // 
        // thickness = 0.05
        // step(d.x, thickness) == step(-0.3, 0.05)
        // "are we closer than thickness to the center?"
        // 
        // step(0.3, 0.05) → Is 0.05 ≥ 0.3? → No → result is 0.0
        // step(0.1, 0.05) → Is 0.05 ≥ 0.1? → No → result is 0.0
        //
        // And example where we'd get 1.0:
        // cp = vec2f(0.375 / 0.25, 0.625 / 0.25) == vec2f(1.5, 1.5)
        // fract(cp) == vec2f(0.5)
        // d = abs(vec2f(0.5) - 0.5) == vec2f(0.0)
        let line = step(d.x, thickness) + step(d.y, thickness);

        // // Or to show circles:
        // let dist = length(d)
        // let line = step(dist, thickness) + step(dist, thickness);

        // line can only be 0.0, 1.0 (x or y matches but not both), or 2.0 (x
        // and y match). Idea: how can we use that without clamping to our
        // advantage?
        let o = clamp(line, 0.0, 1.0);

        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_SMOOTH) {
        let cp = p / cell_size;
        var d = abs(fract(cp) - 0.5);
        let dist = length(d);

        let edge = thickness;

        // smoothstep(edge0, edge1, x) returns:
        // - 0.0 when x <= edge0 
        // - 1.0 when x >= edge1 
        // - smooth transition between [0.0, 1.0] when x is between, using a
        //   cubic Hermite interpolation (ease-in-out)
        let line = 1.0 - smoothstep(edge, edge + softness, dist);

        let o = clamp(line, 0.0, 1.0);
        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_RADIAL) {
        let d = length(p) - radius;
        let warp = vec2f(cos(p.y + t), sin(p.x + t)) * d * warp_amt * 10.0;
        let cp = (p + warp) / cell_size;
        let gx = d - abs(fract(cp.x) - 0.5);
        let line = step(gx, thickness);
        let o = clamp(line, 0.0, 1.0);
        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_WARPED) {
        let dist = length(p);
        let cp = p / cell_size;
        let warped = (cos(cp) * (warp_amt * 10.0 * sin(dist + t * 1.5)) + t) * 
            0.5 + 0.5;
        let d = length(abs(fract(warped) - 0.5));
        let edge = thickness;
        let line = 1.0 - smoothstep(edge - softness, edge + softness, dist - d);
        var o = vec3f(line);

        o.r = o.r + (1.0 - dist - d) + 0.5;
        o.g = o.g + (1.0 - dist - d) + 0.3;
        o.b = o.b + (1.0 - dist - d) + 0.2;

        o = smoothstep(
            vec3f(a1, a2, a3) - 0.4, 
            vec3f(a1, a2, a3) + 0.4, o
        );

        let steps = 3.0;
        o = o * floor(o * steps) / steps;

        return vec4f(vec3f(o), 1.0);
    }

    // Following along at: https://michaelwalczyk.com/blog-ray-marching.html
    if (mode == RAY_MARCH) {
        // z is kind of like fov (field of view)
        let cam_pos = vec3f(0.0, 0.0, -2.0);
        var ray_origin = cam_pos;
        var ray_direction = vec3f(p, 1.0);
        let shaded_color = ray_march(p, ray_origin, ray_direction);
        return vec4f(shaded_color, 1.0); 
    }

    return vec4f(1.0);
}


fn ray_march(p: vec2f, ray_origin: vec3f, ray_direction: vec3f) -> vec3f {
    let t = params.a.z * 0.25;

    var total_distance_traveled = 0.0;
    let steps = 64;
    let min_hit_distance = 0.001;
    let max_trace_distance = 1000.0;

    for (var i = 0; i < steps; i++) {
        var current_position = 
            ray_origin + total_distance_traveled * ray_direction;

        // let xz = rotate(current_position.xz, current_position.y * 0.3 + t);
        // current_position.x = xz.x;
        // current_position.z = xz.y;
        // let yz = rotate(current_position.yz, current_position.y * 0.5 + t);
        // current_position.y = yz.x;
        // current_position.z = yz.y;

        let distance_to_closest = map_the_world(current_position);

        if (distance_to_closest < min_hit_distance) {
            let normal = calculate_normal(current_position);
            let light_position = vec3f(5.0, -2.0, 2.0);
            let direction_to_light = 
                normalize(current_position - light_position);
            let diffuse_intensity = max(0.02, dot(normal, direction_to_light));
            // return normal * 0.5 + 0.5;
            // return (normal * 0.5 + 0.5) * diffuse_intensity;
            // return (vec3f(0.33, 0.4, 1.0) * diffuse_intensity);
            return (vec3f(0.9) * diffuse_intensity);
        }

        if (total_distance_traveled > max_trace_distance) {
            break;
        }
        
        total_distance_traveled += distance_to_closest;
    }

    let noise = fbm(p);
    return vec3f(noise - 0.45, noise - 0.15, noise);
}

fn calculate_normal(p: vec3f) -> vec3f {
    let step = vec3f(0.001, 0.0, 0.0);

    let gx = map_the_world(p + step.xyy) - map_the_world(p - step.xyy);
    let gy = map_the_world(p + step.yxy) - map_the_world(p - step.yxy);
    let gz = map_the_world(p + step.yyx) - map_the_world(p - step.yyx);

    let normal = vec3f(gx, gy, gz);

    return normalize(normal);
}

fn map_the_world(p: vec3f) -> f32 {
    let warp_amt = params.c.x;
    let softness = params.c.y;

    let freq = 5.0;
    let noise = fbm(p.xy) * warp_amt * 0.025;
    let wave = sin(freq * p);
    let product = wave.x * wave.y * wave.z;
    let displacement = (product + noise) *  warp_amt;

    let sdf1 = distance_from_sphere(p, vec3f(0.0));
    let sdf2 = distance_from_sphere(p, vec3f(0.0)) - 0.0618;

    return sdf1 + displacement;
    // return max(sdf1, -sdf2) + displacement;
    // return smax(sdf1, -sdf2, softness) + displacement;
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

fn rotate(v: vec2f, a: f32) -> vec2f {
    let s = sin(a);
    let c = cos(a);
    return vec2f(
        c * v.x - s * v.y,
        s * v.x + c * v.y,
    );
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

