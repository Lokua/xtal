// "Forked" from https://www.shadertoy.com/view/tXscWn

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, time, scale_factor 
    a: vec4f,
    // t_mult, c, s, weight
    b: vec4f,
    // posterize_steps, fractalize, t_loop, t_mix
    c: vec4f,
    // rot_c, rot_s, r, g
    d: vec4f,
    // b, ...
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
    let t = params.a.z * params.b.x;
    var scale_factor = params.a.w;
    let weight = params.b.w;
    let posterize_steps = params.c.x;
    let fractalize = params.c.y;
    let t_loop = params.c.z * params.b.x;
    let t_mix = params.c.w;
    let rot_c = params.d.x;
    let rot_s = params.d.y;
    let c = select(
        params.b.y, 
        cos(params.b.y * mix(t, t_loop, t_mix) * 0.5), 
        bool(rot_c)
    );
    let s = select(
        params.b.z, 
        cos(params.b.z * mix(t, t_loop, t_mix) * 0.5), 
        bool(rot_s)
    );
    let r = params.d.z;
    let g = params.d.w;
    let b = params.e.x;
    
    var p = correct_aspect(position);
    var n = p * fractalize;
    
    var acc = 0.0;
    
    // 2D rotation matrix
    let m = mat2x2(c, s, -s, c);
    
    // Neural noise: 33 iterations
    for (var j = 0.0; j < 33.0; j += 1.0) {
        p = m * p;
        n = m * n;
        
        let q = p * scale_factor + n - mix(t, t_loop, t_mix);
        acc += dot(cos(q), vec2f(weight)) / scale_factor;
        n += sin(q);
        scale_factor *= 1.2;
    }
    
    let l = length(p);

    // Create color from accumulated value 'a' with phase offsets
    let base_color = 0.5 + 0.5 * cos(acc + acc + vec3f(r, g, b));

    // Radial falloff based on distance from center
    let radial_falloff = 1.0 / (1.0 + l * 0.5);

    // Vignette effect (darker at edges)
    let vignette = 1.0 - l * l * 0.0001;

    // Combine all effects
    var color = base_color * radial_falloff * vignette;

    let ps = posterize_steps;
    color.x = max(0.01, floor(color.x * ps) / ps);
    color.y = max(0.01, floor(color.y * ps) / ps);
    color.z = max(0.01, floor(color.z * ps) / ps);

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

