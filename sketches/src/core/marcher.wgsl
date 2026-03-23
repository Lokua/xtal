//! "Forked" from https://www.shadertoy.com/view/lcfXD8

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, time, ripple_amp 
    a: vec4f,
    // ripple_freq, inner_mult, outer_mult, outer_mult_2
    b: vec4f,
    // radial_freq, v_base, fm, fm_range
    c: vec4f,
    // fm_base, t_mult, detail, increment
    d: vec4f,
    // show_ripple, show_swirl, show_pulse, show_v
    e: vec4f,
    // show_fm, gyr_alg, UNUSED, pos_x
    f: vec4f,
    // pos_y, r, g, b
    g: vec4f,
    // colorize, ...
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
    let pos_x = params.f.w;
    let pos_y = params.g.x;
    let r = params.g.y;
    let g = params.g.z;
    let b = params.g.w;
    let colorize = params.h.x;

    let pos = correct_aspect(position);
    
    var d = 0.0;
    var dd = increment;
    // Camera position
    var p = vec3f(0.0, 0.0, t / 8.0);
    // Ray direction
    var rd = normalize(vec3f(pos.xy - vec2f(pos_x, pos_y), 1.0));
    
    for (var i = 0.0; i < 90.0 && dd > 0.001 && d < 2.0; i += 1.0) {
        d += dd;
        p += rd * d;
        dd = map(p) * detail;
    }
    
    var n = norm(p);
    var c = n.x + n.y;
    c *= SS(0.9, 0.15, 1.0 / d);
    n = n * 0.5 + 0.5;

    let bw = vec3f(c);

    let colorized = vec3f(
        n.x * r, 
        n.y * g, 
        c * b, 
    ) * c;

    let color = mix(bw, colorized, colorize);

    return vec4f(color, 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn SS(a: f32, b: f32, c: f32) -> f32 {
    return smoothstep(a - b, a + b, c);
}

fn gyr(p: vec3f) -> f32 {
    let gyr_alg = params.f.y;
    let a = sin(p.xyz);
    let b = cos(p.zxy);
    return mix(dot(a, b), length(a - b), gyr_alg);
}

fn map(p: vec3f) -> f32 {
    let t = params.a.z; 
    let ripple_amp = params.a.w;
    let ripple_freq = params.b.x;
    let inner_mult = params.b.y;
    let outer_mult = params.b.z;
    let outer_mult_2 = params.b.w;
    let radial_freq = params.c.x;
    let v_base = params.c.y;
    let fm = params.c.z;
    let fm_range = params.c.w;
    let fm_base = params.d.x;
    let show_ripple = params.e.x;
    let show_swirl = params.e.y;
    let show_pulse = params.e.z;
    let show_v = params.e.w;
    let show_fm = params.f.x;
    let pos_x = params.f.w;
    let pos_y = params.g.x;
    
    let ripple = (1.0 + ripple_amp * sin(p.y * ripple_freq));
    let inner_swirl = gyr(p * inner_mult); 
    let outer_swirl = gyr(p * outer_mult + outer_mult_2 * inner_swirl); 
    let radial_pulse = (1.0 + sin(t + length(p.xy) * radial_freq));
    let vertical_wave = v_base * sin(t * 0.15 + p.z * 5.0 + p.y);
    let freq_modulator = sin(t * fm + p.z * 3.0) * 
        fm_range + (fm_range * 0.666);
    let hi_freq_swirl = fm_base + gyr(p * freq_modulator);

    var o = 1.0;
    if show_ripple == 1.0 {
        o *= ripple;
    }
    if show_swirl == 1.0 {
        o *= outer_swirl;
    }
    if show_pulse == 1.0 {
        o *= radial_pulse;
    }

    var oo = 1.0;
    if show_v == 1.0 {
        oo *= vertical_wave;
    }
    if show_fm == 1.0 {
        oo *= hi_freq_swirl;
    }

    return o + oo;
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
