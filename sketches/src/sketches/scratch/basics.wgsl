const GRADIENT: i32 = 0;
const GRADIENT_STEPPED: i32 = 1;
const CIRCLE: i32 = 2;
const CIRCLE_STEPPED: i32 = 3;
const GRID: i32 = 4;
const GRID_WARPED: i32 = 5;
const GRID_RADIAL: i32 = 6;
const GRID_MULTI: i32 = 7;

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
    // radius, thickness, step_size, grid_size
    b: vec4f,
    // warp_amt, ...
    c: vec4f,
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
    let grid_size = params.b.w;
    let warp_amt = params.c.x;

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
        let coord = p / grid_size;
        let gx = abs(fract(coord.x) - 0.5);
        let gy = abs(fract(coord.y) - 0.5);
        let line = step(gx, thickness) + step(gy, thickness);
        let o = clamp(line, 0.0, 1.0);
        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_WARPED) {
        let warp = vec2f(sin(p.y + t), cos(p.x + t)) * warp_amt;
        let coord = (p + warp) / grid_size;
        let gx = abs(fract(coord.x) - 0.5);
        let gy = abs(fract(coord.y) - 0.5);
        let line = step(gx, thickness) + step(gy, thickness);
        let o = clamp(line, 0.0, 1.0);
        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_RADIAL) {
        let d = length(p) - radius;
        let warp = vec2f(sin(p.y + t), cos(p.x + t)) * 
            (d * step_size) * 
            warp_amt;
        let coord = (p + warp) / grid_size;
        let gx = abs(fract(coord.x) - 0.5);
        let gy = abs(fract(coord.y) - 0.5);
        let line = step(gx, thickness);
        let o = clamp(line, 0.0, 1.0);
        return vec4f(vec3f(o), 1.0);
    }

    if (mode == GRID_MULTI) {
        // let p0 = grid_radial(p);
        let p1 = grid_radial(p + 0.5);
        let p2 = grid_radial(p - 0.5);
        let o = min(p1, p2);
        // let o = min(p0, min(p1, p2));
        return vec4f(vec3f(o), 1.0);
    }

    return vec4f(1.0);
}

fn grid_radial(p: vec2f) -> f32 {
    let t = params.a.z * 0.25;
    let mode = i32(params.a.w);
    let radius = params.b.x;
    let thickness = params.b.y;
    let step_size = params.b.z;
    let grid_size = params.b.w;
    let warp_amt = params.c.x;

    let d = length(p) - radius;
    let warp = vec2f(sin(p.y + t), cos(p.x + t)) * 
        (d * step_size) * 
        warp_amt;

    let coord = (p + warp) / grid_size;
    let gx = abs(fract(coord.x) - 0.5);
    let gy = abs(fract(coord.y) - 0.5);
    let line = step(gx, thickness);

    let o = clamp(line, 0.0, 1.0);

    return o;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

