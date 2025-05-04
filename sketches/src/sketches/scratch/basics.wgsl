const GRADIENT: i32 = 0;
const GRADIENT_STEPPED: i32 = 1;
const CIRCLE: i32 = 2;
const CIRCLE_STEPPED: i32 = 3;
const GRID: i32 = 4;
const GRID_SMOOTH: i32 = 5;
const GRID_RADIAL: i32 = 6;
const GRID_WARPED: i32 = 7;

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

    return vec4f(1.0);
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

