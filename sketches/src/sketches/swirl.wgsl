//! Originally forked from https://www.shadertoy.com/view/lcfXD8

const EDGE_MODE_R = 0.0;
const EDGE_MODE_G = 1.0;
const EDGE_MODE_B = 2.0;
const EDGE_MODE_RG = 3.0;
const EDGE_MODE_RB = 4.0;
const EDGE_MODE_GB = 5.0;
const EDGE_MODE_RGB = 6.0;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, time, gyr_mix 
    a: vec4f,
    // gyr_b_amt, inner_mult, outer_mult, outer_meta
    b: vec4f,
    // v_base, v_y_offs, animate_pos_range, pos_x_anim
    c: vec4f,
    // pos_y_anim, t_mult, detail, increment
    d: vec4f,
    // UNUSED, UNUSED, UNUSED, UNUSED
    e: vec4f,
    // UNUSED, UNUSED, UNUSED, pos_x
    f: vec4f,
    // pos_y, r, g, b
    g: vec4f,
    // colorize, edge_thresh, edge_mode, edge_mix
    h: vec4f,
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
    let t_mult = params.d.y;
    let t = (params.a.z / 2.0) * t_mult;
    let detail = params.d.z;
    let increment = params.d.w;
    let r = params.g.y;
    let g = params.g.z;
    let b = params.g.w;
    let colorize = params.h.x;
    let edge_thresh = params.h.y;
    let edge_mode = params.h.z;
    
    var pos_x = params.f.w;
    var pos_y = params.g.x;
    let animate_pos_range = params.c.z;
    if animate_pos_range == 1.0 {
        pos_x = params.c.w;
        pos_y = params.d.x;
    }

    let pos = correct_aspect(position);
    
    var d = 0.0;
    var dd = increment;
    var p = vec3f(0.0, 0.0, t / 8.0);
    var ray_dir = normalize(vec3f(pos.xy - vec2f(pos_x, pos_y), 1.0));
    
    for (var i = 0.0; i < 90.0 && dd > 0.001 && d < 2.0; i += 1.0) {
        d += dd;
        p += ray_dir * d;
        dd = map(p) * detail;
    }
    
    var n = norm(p);
    var c = n.x + n.y;
    c *= SS(0.9, 0.15, 1.0 / d);
    n = n * 0.5 + 0.5;
    let cz = (ray_dir.z * 0.5 + 0.5);

    let bw = vec3f(c);
    var colorized = vec3f(n.x * r, n.y * g, (cz + c) * b) * c;

    if c >= 0.0 && c <= edge_thresh {
        colorized = paint_edges(colorized);
    }
    
    let color = mix(bw, colorized, colorize);
    
    return vec4f(color, 1.0);
}

fn paint_edges(cd: vec3f) -> vec3f {
    let edge_mode = params.h.z;
    let edge_mix = params.h.w;
    var result = cd;
    
    if edge_mode == EDGE_MODE_R {
        result.r = 1.0 - cd.r;
    } else if edge_mode == EDGE_MODE_G {
        result.g = 1.0 - cd.g;
    } else if edge_mode == EDGE_MODE_B {
        result.b = 1.0 - cd.b;
    } else if edge_mode == EDGE_MODE_RG {
        result.r = 1.0 - cd.r;
        result.g = 1.0 - cd.g;
    } else if edge_mode == EDGE_MODE_RB {
        result.r = 1.0 - cd.r;
        result.b = 1.0 - cd.b;
    } else if edge_mode == EDGE_MODE_GB {
        result.g = 1.0 - cd.g;
        result.b = 1.0 - cd.b;
    } else if edge_mode == EDGE_MODE_RGB {
        result.r = 1.0 - cd.r;
        result.g = 1.0 - cd.g;
        result.b = 1.0 - cd.b;
    }
    
    return mix(cd, result, edge_mix);
}

fn gyr(p: vec3f) -> f32 {
    let gyr_mix = params.a.w;
    let gyr_b_amt = params.b.x;
    let a = sin(p.xyz);
    let b = mix(cos(p.zxy), tanh(p.zxy * gyr_b_amt), gyr_mix);
    return dot(a, b);
}

fn map(p: vec3f) -> f32 {
    let t = params.a.z; 
    let inner_mult = params.b.y;
    let outer_mult = params.b.z;
    let outer_meta = params.b.w;
    let fm = params.c.z;
    let fm_range = params.c.w;
    let fm_base = params.d.x;
    let show_ripple = params.e.x;
    let pos_x = params.f.w;
    let pos_y = params.g.x;
    let v_base = params.c.x;
    let v_y_offs = params.c.y;
    
    // let sp = vec3f(p.x * 2.0 + abs(p.z), p.y * 5.0 - abs(p.z), p.z * 1.0);
    // let inner_swirl = gyr(sp * inner_mult); 
    let inner_swirl = gyr(p * inner_mult); 
    let outer_swirl = gyr(p * outer_mult + outer_meta * inner_swirl); 
    let vertical_wave = v_base * sin((t * 0.1666666) + (p.z * v_y_offs) + p.y);

    return outer_swirl + vertical_wave;
}

fn norm(p: vec3f) -> vec3f {
    let m = map(p);
    let n = 40.0;
    let d = vec2f(0.06 + 0.06 * sin(p.z), 0.0);
    return map(p) - vec3f(
        map(p - d.xyy),
        map(p - d.yxy),
        map(p - d.yyx)
    );
}

fn SS(a: f32, b: f32, c: f32) -> f32 {
    return smoothstep(a - b, a + b, c);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}