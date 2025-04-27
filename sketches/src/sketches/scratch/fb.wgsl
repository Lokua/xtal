struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
};

struct Params {
    // w, h, t, feedback
    a: vec4f,
    // delay, sample_offs_x, sample_offs_y, fract_mix
    b: vec4f,
    // radius_animation, center_range, animate_radius, radius
    c: vec4f,
    // wrap_offs_x, wrap_offs_y, grain_mix, UNUSED
    d: vec4f,
    e: vec4f,
    e: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var feedback_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = out.position.xy;
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) pos: vec2f,
    @location(1) uv: vec2f
) -> @location(0) vec4f {
    let t = params.a.z;
    let fb = params.a.w;
    let sample_offs_x = params.b.y;
    let sample_offs_y = params.b.z;
    let fract_mix = params.b.w;
    let radius_animation = params.c.x;
    let center_range = params.c.y;
    let animate_radius = params.c.z == 1.0;
    let radius_slider = params.c.w;
    let wrap_offs_x = params.d.x == 1.0;
    let wrap_offs_y = params.d.y == 1.0;
    let grain_mix = params.d.z;
    let grid_size_x = params.e.x;
    let grid_size_y = params.e.y;
    let grid_spacing = params.e.z;
    let circle_scale = params.e.w;

    let p = correct_aspect(pos);

    let offs_x = uv.x + (cos(t) * sample_offs_x);
    let offs_y = uv.y + (sin(t) * sample_offs_y);

    let fb_sample = textureSample(
        feedback_texture, 
        source_sampler, 
        vec2f(
            select(offs_x, modulo(offs_x, 1.0), wrap_offs_x),
            select(offs_y, modulo(offs_y, 1.0), wrap_offs_y),
        )
    );
    
    let center = vec2f(sin(t), cos(t) * 0.25) * center_range;
    let radius = select(radius_slider, radius_animation, animate_radius);

    let dist = length(p - center);
    
    var color = vec3f(0.0);

    if (dist < radius) {
        color = vec3f(
            n(sin(t * 0.1)), 
            n(sin(t * 0.2)), 
            n(sin(t * 0.3)), 
        );
        color = film_grain(color, p, grain_mix);
    } 
    
    color = mix(color, fb_sample.rgb, fb);
    color += (color * 0.125);
    color = mix(color, fract(color.grg), fract_mix);

    return vec4f(color, 1.0);
}

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = random_v2(p);
    return clamp(color + (random - 0.25) * intensity, vec3f(0.0), vec3f(1.0));
}

fn random_v2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn n(x: f32) -> f32 {
    return x * 0.5 + 0.5;
}

fn mod_1(x: f32) -> f32 {
    return modulo(x, 1.0);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn mod_v3(x: vec3f, y: vec3f) -> vec3f {
    return x - y * floor(x / y);
}

