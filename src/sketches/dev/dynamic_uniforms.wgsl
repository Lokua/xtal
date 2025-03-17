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
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
    g: vec4f,
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
    let wave_phase = params.b.x;
    let wave_dist = params.b.y;
    let wave_x_freq = params.b.z;
    let wave_y_freq = params.b.w;
    let hue = params.c.y;
    let color_amt = params.c.x;
    let color_freq = params.c.y;
    let color_phase = params.c.z;
    let r_amp = params.c.w;
    let g_amp = params.d.x;
    let b_amp = params.d.y;
    let color_shift = params.d.z;
    let color_invert = params.d.w;

    let p = correct_aspect(position);
    let d = length(p);

    let wave1 = cos(d * wave_dist - wave_phase);
    let wave2 = cos(p.x * wave_x_freq);
    let wave3 = sin(p.y * wave_y_freq);
    let waves = (wave1 + wave2 + wave3);

    var c_val = ease(0.5 + (cos(waves) * 0.5));
    
    let color_wave = waves * color_freq + color_phase;
    
    let r = c_val + r_amp * sin(color_wave) * color_amt;
    let g = c_val + g_amp * sin(color_wave + color_shift) * color_amt;
    let b = c_val + b_amp * sin(color_wave + color_shift * 2.0) * color_amt;

    var color = vec3f(r, g, b);
    if color_invert == 1.0 {
        color = 1.0 - color;
    }
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

fn ease(t: f32) -> f32 {
    return t * t * t;
}

fn hsl_to_rgb(hsl: vec3f) -> vec3f {
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;
    
    let c = (1.0 - abs(2.0 * l - 1.0)) * s;
    let x = c * (1.0 - abs(fract(h * 6.0) - 3.0 - 1.0));
    let m = l - c / 2.0;
    
    var rgb: vec3f;
    if (h < 1.0/6.0) {
        rgb = vec3f(c, x, 0.0);
    } else if (h < 2.0 / 6.0) {
        rgb = vec3f(x, c, 0.0);
    } else if (h < 3.0 / 6.0) {
        rgb = vec3f(0.0, c, x);
    } else if (h < 4.0 / 6.0) {
        rgb = vec3f(0.0, x, c);
    } else if (h < 5.0 / 6.0) {
        rgb = vec3f(x, 0.0, c);
    } else {
        rgb = vec3f(c, 0.0, x);
    }
    
    return rgb + m;
}

