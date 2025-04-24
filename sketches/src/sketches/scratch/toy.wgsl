// Forked from https://twigl.app/?ol=true&ss=-OOTrxZCfc7skbElm-8u
// Originally written by https://x.com/XorDev (?)

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, app.time, cam
    a: vec4f,
    // brightness, rotation, t_disp, depth
    b: vec4f,
    // r_mult, g_mult, b_mult, n_lines
    c: vec4f,
    // alg_mix, x_offs, y_offs, swizzle
    d: vec4f,
    // t_mult, ...
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
    let t_mult = params.e.x;
    let t = params.a.z * t_mult;
    let cam = params.a.w;
    let brightness = params.b.x;
    let rotation = params.b.y;
    let t_disp = params.b.z;
    let depth = params.b.w;
    let r_mult = params.c.x;
    let g_mult = params.c.y;
    let b_mult = params.c.z;
    let n_lines = params.c.w;
    let alg_mix = params.d.x;
    let x_offs = params.d.y;
    let y_offs = params.d.z;
    let swizzle = params.d.w;

    let pos = correct_aspect(position);
    var r = vec3f(pos, 1.0);
    var o = vec4f(0.0);
    
    var p: vec3f;
    var z = 0.0;

    for (var i = 0.0; i < n_lines; i += 1.0) {
        p = z * normalize(vec3f(pos, cam));
        p.z += t;

        let a = z * rotation + t * 0.1;
        
        // 2D rotation matrix application
        let cos_a = mix(cos(a), 1.0 / tan(a) + x_offs, alg_mix);
        let sin_a = mix(sin(a), tan(a) + y_offs, alg_mix);
        let temp_x = p.x * cos_a - p.y * sin_a;
        let temp_y = p.x * sin_a + p.y * cos_a;
        p.x = temp_x;
        p.y = temp_y;

        // Distance estimator
        let swizzled = select(
            vec3f(p.z, p.x, p.y), 
            vec3f(p.x, p.y, p.z), 
            swizzle == 1.0
        );
        let pattern = cos(swizzled + p.z - t * t_disp);
        let d = length(cos(p + pattern).xy) / depth;
        z += d;

        o += (sin(p.x + t + vec4f(0.0, 2.0, 3.0, 0.0)) + 1.0) / d;
    }

    o = tanh(o / pow(brightness, 2.0));
    o.r *= r_mult;
    o.g *= g_mult;
    o.b *= b_mult;

    return vec4f(o.xyz, 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

