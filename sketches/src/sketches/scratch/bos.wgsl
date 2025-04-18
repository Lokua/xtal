const PI: f32 = 3.14159265359;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec2f,
    a: f32,
    b: f32,
    t: f32,
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
    var pos = position;
    pos.x *= params.resolution.x / params.resolution.y;
    
    // return bos_05a(pos);
    // return bos_05b(pos);
    // return bos_05c(pos);
    // return bos_05d(pos);
    // return bos_07a(pos);
    // return bos_07b(pos);
    // return bos_07c(pos);
    // return bos_07d(pos);
    return bos_07e(pos);
    // return bos_07f(pos);
    // return bos_07g(pos);
    // return bos_07h(pos);
    // return bos_07i(pos);
}

// Shaping Functions: linear interpolation
fn bos_05a(pos: vec2f) -> vec4f {
    let uv = pos * 0.5 + 0.5;

    let y = uv.x;
    var color = vec3f(y);

    // Plot
    let pct = smoothstep(params.a, 0.0, abs(uv.y - uv.x));
    color = (1.0 - pct) * color + pct * vec3f(0.0, 1.0, 0.0);

    return vec4f(color, 1.0);
}

// Shaping Functions: curved
fn bos_05b(pos: vec2f) -> vec4f {
    let uv = pos * 0.5 + 0.5;

    let y = pow(uv.x, params.b * 10.0);

    var color = vec3f(y);

    // Plot
    let pct = 
        smoothstep(y - params.a, y, uv.y) - 
        smoothstep(y, y + params.a, uv.y);

    color = (1.0 - pct) * color + pct * vec3f(0.0, 1.0, 0.0);

    return vec4f(color, 1.0);
}

// Shaping Functions: step
fn bos_05c(pos: vec2f) -> vec4f {
    let uv = pos * 0.5 + 0.5;

    // Step will return 0.0 unless the value is over 0.5,
    // in that case it will return 1.0
    let y = step(params.b, uv.x);

    var color = vec3f(y);

    // Plot
    let pct = 
        smoothstep(y - params.a, y, uv.y) - 
        smoothstep(y, y + params.a, uv.y);

    color = (1.0 - pct) * color + pct * vec3f(0.0, 1.0, 0.0);

    return vec4f(color, 1.0);
}

// Shaping Functions: smoothstep
fn bos_05d(pos: vec2f) -> vec4f {
    let uv = pos * 0.5 + 0.5;

    // Smooth interpolation between b and 0.9
    // let y = smoothstep(params.b, 0.9, uv.x);
    // Vertical "cut"
    let y = smoothstep(0.2, 0.5, uv.x) - smoothstep(0.5, 0.8, uv.x);

    var color = vec3f(y);

    // Plot
    let pct = 
        smoothstep(y - params.a, y, uv.y) - 
        smoothstep(y, y + params.a, uv.y);

    color = (1.0 - pct) * color + pct * vec3f(0.0, 1.0, 0.0);

    return vec4f(color, 1.0);
}

// Distance Fields
fn bos_07a(pos: vec2f) -> vec4f {
    // Calculate distance from center
    let d = length(pos);

    // Filled shape
    return vec4f(vec3f(step(d, params.a)), 1.0);
}

// Distance Fields (cont)
fn bos_07b(pos: vec2f) -> vec4f {
    // abs(pos) folds the space into the first quadrant
    // Subtracting params.a creates a square boundary
    // This is "distance to rectangle's border" function
    let d = length(abs(pos) - params.a);


    // Creates a cross
    // let d = length(min(abs(pos) - params.a, vec2f(0.0)));

    // Creates a square
    // let d = length(max(abs(pos) - params.a, vec2f(0.0)));

    // Other
    // let d = distance(pos, vec2(-params.a)) + distance(pos, vec2(params.a));
    // let d = distance(pos, vec2(-params.a)) * distance(pos, vec2(params.a));
    // let d = min(distance(pos, vec2(-params.a)), distance(pos, vec2(params.a)));
    // let d = max(distance(pos, vec2(-params.a)), distance(pos, vec2(params.a)));
    // let d = pow(distance(pos, vec2(-params.a)), distance(pos, vec2(params.a)));

    // Visualize raw distance field
    // return vec4f(vec3f(d), 1.0);

    // Draw a repeating distance field 
    return vec4f(vec3f(fract(d * (params.b * 50.0))), 1.0);

    // Draw rings / outlines
    // let ring_width = params.b;
    // return vec4f(
    //     vec3f(step(0.3, d) * step(d, 0.3 + ring_width)), 
    //     1.0
    // );

    // Gradient version of above
    // return vec4f(
    //     vec3f(smoothstep(0.3, 0.4, d) * smoothstep(0.6, 0.5, d)), 
    //     1.0
    // );
}

// Distance Fields Challenge: animate
fn bos_07c(pos: vec2f) -> vec4f {
    let scale = params.t * params.b + (1.0 - params.b);
    let d = length(pos * scale);
    return vec4f(vec3f(step(d, params.a)), 1.0);
}

// Distance Fields Challenge: move + more
fn bos_07d(pos: vec2f) -> vec4f {
    if pos.x < 0.0 {
        let dx = (params.b - 0.5) * 2.0;
        let moved = vec2(pos.x - dx, pos.y);
        let d = length(moved);

        return vec4f(vec3f(step(d, params.a)), 1.0);
    }

    if pos.y < 0.0 {
        let dy = (params.a - 0.5) * 2.0;
        let moved = vec2(pos.x, pos.y - dy);
        let d = length(moved);

        return vec4f(vec3f(step(d, params.b)), 1.0);
    }

    return vec4f(0.0, 0.0, 0.0, 1.0);
}

// Polar Shapes
fn bos_07e(pos: vec2f) -> vec4f {
    let r = length(pos) * 2.0;
    let a = atan2(pos.y, pos.x);
    let v = trunc(params.a * 20.0);
    // let f = cos(a * v);
    // let f = abs(cos(a * v));
    // let f = abs(cos(a * v)) * 0.75 + 0.9;
    let f = abs(cos(a * 12.0) * sin(a * v)) * 0.8 + 0.1;
    // let f = smoothstep(-0.5, 1.0, cos(a * 10.0)) * 0.2 + 0.5;
    let color = vec3f(1.0 - smoothstep(f, f + 0.02, r));
    return vec4(color, 1.0);
}

// Polar Shapes: Challenge - holes
fn bos_07f(pos: vec2f) -> vec4f {
    let r = length(pos) * 2.0;
    let a = atan2(pos.y, pos.x);
    let v = trunc(params.a * 20.0);
    
    // First shape (black)
    // let f1 = abs(cos(a * v));
    let f1 = abs(cos(a * v)) * 0.5 + 0.9;
    let shape1 = 1.0 - smoothstep(f1, f1 + params.b, r);
    
    // Second shape (white)
    let f2 = abs(cos(a * v)) * 0.75 + 0.9;
    let shape2 = 1.0 - smoothstep(f2, f2 + params.b, r);
    
    // Combine them - shape1 in black, shape2 in white
    let color = vec3f(shape2 * (1.0 - shape1));
    // Or alternatively: let color = vec3f(select(shape2, 0.0, shape1 > 0.0));
    
    return vec4f(color, 1.0);
}

// Polar Shapes: Challenge - plot just the contour
fn bos_07g(pos: vec2f) -> vec4f {
    let r = length(pos) * 2.0;
    let a = atan2(pos.y, pos.x);
    let f = smoothstep(-0.5, 1.0, cos(a * 10.0)) * 0.2 + params.a;
    // let color = vec3f(smoothstep(params.b, 0.0, abs(r - f)));
    let color = vec3f(step(abs(r - f), params.b));
    return vec4(color, 1.0);
}

// Comnbining Powers
fn bos_07h(pos: vec2f) -> vec4f {
    let n = trunc(params.a * 12.0);
    let a = atan2(pos.y, pos.x) + PI;
    let r = 2.0 * PI / f32(n);
    let d = cos(floor(0.5 + a / r) * r - a) * length(pos);
    let color = vec3f(1.0 - smoothstep(0.4, 0.41, d));
    return vec4(color, 1.0);
}

// Comnbining Powers: Challenge - mixing distance fields
fn bos_07i(pos: vec2f) -> vec4f {
    let d1 = polygon_distance(pos, u32(trunc(params.a * 12.0)));
    let d2 = polygon_distance(pos, u32(trunc(params.b * 12.0)));
    let d = min(d1, d2);
    let color = vec3f(1.0 - smoothstep(0.4, 0.41, d));
    return vec4(color, 1.0);
}

fn polygon_distance(pos: vec2f, n: u32) -> f32 {
    let a = atan2(pos.y, pos.x) + PI;
    let r = 2.0 * PI / f32(n);
    let d = cos(floor(0.5 + a / r) * r - a) * length(pos);
    return d;
}
