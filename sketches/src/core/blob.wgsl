const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const STANDARD_LUMINANCE: vec3f = vec3f(0.2126, 0.7152, 0.0722);
var<private> OFFSETS: array<vec2f, 4> = array<vec2f, 4>(
    vec2f(-1.0, 0.0),
    vec2f(1.0, 0.0),
    vec2f(0.0, -1.0),
    vec2f(0.0, 1.0)
);

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
};

struct Params {
    // x, y, edge_mix, edge_size
    resolution: vec4f,
    // t1, t2, t3, t4
    a: vec4f,
    // invert, center_size, smoothness, color_mix
    b: vec4f,
    // t_long, center_y, outer_scale_animation_a, center_size
    c: vec4f,
    // feedback, outer_size, outer_scale_animation_mix, outer_scale_animation_b
    d: vec4f,
    // rot_angle, bd, clamp_mix, clamp_max
    e: vec4f,
    // chromatic_feedback_spread, unused, crt_glitch_phase, unused
    f: vec4f,
    // unused
    g: vec4f,
    // unused, crt_scanline, crt_jitter, unused
    h: vec4f,
    // unused, crt_phase_mix, unused, unused
    i: vec4f,
    j: vec4f,
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
    out.pos = vert.position;
    out.uv = vec2f(out.pos.x, -out.pos.y) * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) position: vec2f, 
    @location(1) uv: vec2f
) -> @location(0) vec4f {
    // Animation times in quadrant order; t1=q1 and so on clockwise
    let t1 = params.a.x;
    let t2 = params.a.y;
    let t3 = params.a.z;
    let t4 = params.a.w;
    let invert_color = params.b.x;
    let smoothness = params.b.y;
    let blur = params.b.z;
    let color_mix = params.b.w;
    let t_long = params.c.x;
    let center_y = params.c.y;
    let outer_scale_animation_a = params.c.z;
    let outer_size = 1.0 - params.d.y;
    let outer_scale_animation_mix = params.d.z;
    let outer_scale_animation_b = params.d.w;
    let center_size = params.c.w;
    let rot_angle = params.e.x;
    let bd = params.e.y;
    let clamp_min = params.e.z;
    let clamp_max = params.e.w;
    let feedback = params.d.x;
    let edge_mix = params.resolution.z;
    let crt_glitch = params.f.z;
    let crt_scanline = params.h.y;
    let crt_jitter = params.h.z;
    let crt_phase_mix = params.i.y;

    var p = correct_aspect(position);

    let os = mix(
        outer_scale_animation_a, 
        outer_scale_animation_b, 
        outer_scale_animation_mix
    );

    let p1xt = mix(t4, t1, outer_scale_animation_mix);
    let p1yt = mix(t3, t1, outer_scale_animation_mix);
    let p2xt = mix(t2, t2, outer_scale_animation_mix);
    let p2yt = mix(t1, t2, outer_scale_animation_mix);
    let p3xt = mix(t4, t3, outer_scale_animation_mix);
    let p3yt = mix(t3, t3, outer_scale_animation_mix);
    let p4xt = mix(t4, t4, outer_scale_animation_mix);
    let p4yt = mix(t3, t4, outer_scale_animation_mix);

    var p1 = vec2f((1.0 - p1xt) * os, (1.0 - p1yt) * os);
    var p2 = vec2f((1.0 - p2xt) * os, (-1.0 + p2yt) * os);
    var p3 = vec2f((-1.0 + p3xt) * os, (-1.0 + p3yt) * os);
    var p4 = vec2f((-1.0 + p4xt) * os, (1.0 - p4yt) * os);

    // center
    var p0 = vec2f(0.0, center_y);

    p1 = clamp_v2(p1, clamp_min, clamp_max);
    p2 = clamp_v2(p2, clamp_min, clamp_max);
    p3 = clamp_v2(p3, clamp_min, clamp_max);
    p4 = clamp_v2(p4, clamp_min, clamp_max);

    let angle = rot_angle * TAU;
    p0 = rotate_point(correct_aspect(p0), angle);
    p1 = rotate_point(correct_aspect(p1), angle);
    p2 = rotate_point(correct_aspect(p2), angle);
    p3 = rotate_point(correct_aspect(p3), angle);
    p4 = rotate_point(correct_aspect(p4), angle);


    let scale = 1.0;
    let d0 = length(p - p0) / (scale * 0.5);
    let d1 = length(p - p1) / scale * outer_size;
    let d2 = length(p - p2) / scale * outer_size;
    let d3 = length(p - p3) / scale * outer_size;
    let d4 = length(p - p4) / scale * outer_size;

    // As outer_size increases, smoothness decreases
    let k = smoothness * outer_size;
    let k_center = k * center_size;
    
    // Mix each corner with the center point
    let mix1 = smin(d1, d0, k_center);
    let mix2 = smin(d2, d0, k_center);
    let mix3 = smin(d3, d0, k_center);
    let mix4 = smin(d4, d0, k_center);

    // Combine all mixed pairs
    let mix12 = smin(mix1, mix2, k);
    let mix34 = smin(mix3, mix4, k);
    let final_mix = smin(mix12, mix34, k);

    let d = final_mix * blur;

    let color_p = vec2f(p.y, p.x);
    var rainbow = vec3f(
        0.5 + 0.5 * sin(color_p.x * 2.0),
        0.5 + 0.5 * cos(color_p.y * 2.0),
        0.5 + 0.5 * sin((color_p.x + color_p.y) * 1.0)
    );
    rainbow = vec3f(rotate_point(rainbow.xy, t_long * 0.0125), rainbow.z);
    let grid_resolution = 700.0 + t_long;
    let grid_color = vec3f(
        0.5 + 0.5 * sin(color_p.x * grid_resolution),
        0.5 + 0.5 * cos(color_p.y * grid_resolution),
        0.5 + 0.5 * sin((color_p.x + color_p.y) * grid_resolution)
    );
    
    let base_color = mix(rainbow, grid_color, color_mix);

    // How much we're in the center
    let center_influence = smoothstep(1.0 - bd, 0.0, d0);
    let center_color = vec3f(1.0, 0.0, 0.5);
    let center_off = 0.0;
    var color = mix(base_color, center_color, center_influence * center_off);
    
    // For areas where d is small (inside circles), use bright colors
    // For areas where d is large (background), fade to darker
    let circle_brightness = smoothstep(1.0, 0.9, d);
    color = color * (0.3 + 0.99 * circle_brightness); 
    color = mix(color, 1.0 - color, invert_color);

    color = edge_detect(uv, color, edge_mix);
    color = chromatic_feedback(uv, color + (color * feedback), feedback);
    color = crt_glitch_phase(
        uv,
        color,
        crt_glitch,
        crt_scanline,
        crt_jitter,
        crt_phase_mix
    );
    
    return vec4f(color, 1.0);
}

fn rotate_point(p: vec2f, angle: f32) -> vec2f {
    let rot_matrix = mat2x2(
        cos(angle), -sin(angle),
        sin(angle), cos(angle)
    );
    return rot_matrix * p;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

fn clamp_v2(p: vec2f, min: f32, max: f32) -> vec2f {
    return vec2f(
        clamp(p.x, min, max), 
        clamp(p.y, min, max)
    );
}

fn chromatic_feedback(uv: vec2f, color: vec3f, mix: f32) -> vec3f {
    let t_long = params.c.x;
    let chromatic_feedback_spread = params.f.x;
    var dx = vec2f(chromatic_feedback_spread, 0.0);
    var dy = vec2f(0.0, chromatic_feedback_spread);
    dx = rotate_point(dx, t_long * 0.0125);
    dy = rotate_point(dy, t_long * 0.0125);
    let r = textureSample(feedback_texture, source_sampler, uv - dx).r;
    let g = textureSample(feedback_texture, source_sampler, uv).g;
    let b = textureSample(feedback_texture, source_sampler, uv + dy).b;
    let feedback_color = vec3f(r, g, b);
    return mix(color, feedback_color, mix);
}

fn crt_glitch_phase(
    uv: vec2f,
    color: vec3f,
    amount: f32,
    scanline_mult: f32,
    jitter_mult: f32,
    phase_mix: f32
) -> vec3f {
    let t_long = params.c.x;
    let dims = params.resolution.xy;

    let centered = uv * 2.0 - 1.0;
    let r2 = dot(centered, centered);
    let barrel = centered;
    var crt_uv = barrel * 0.5 + 0.5;

    let row = floor(crt_uv.y * 120.0);
    let row_noise = fract(sin(row * 12.9898 + t_long * 0.8) * 43758.5453);
    crt_uv.x += (row_noise - 0.5) * 0.02 * amount * jitter_mult;
    crt_uv = clamp(crt_uv, vec2f(0.001, 0.001), vec2f(0.999, 0.999));

    let scan = 0.85 + 0.15 * sin(
        crt_uv.y * dims.y * 1.2 * scanline_mult + t_long * 8.0
    );
    let scan_mix = mix(1.0, scan, amount);

    let split = 0.001 + amount * 0.008 * 2.0;
    let r = textureSample(
        feedback_texture,
        source_sampler,
        clamp(crt_uv + vec2f(split, 0.0), vec2f(0.001), vec2f(0.999))
    ).r;
    let g = textureSample(feedback_texture, source_sampler, crt_uv).g;
    let b = textureSample(
        feedback_texture,
        source_sampler,
        clamp(crt_uv - vec2f(split, 0.0), vec2f(0.001), vec2f(0.999))
    ).b;

    let glitch_color = vec3f(r, g, b);
    let vignette = 1.0;
    let crt_color = color * scan_mix * vignette;
    let phased = mix(crt_color, glitch_color, phase_mix * amount);
    return mix(color, phased, amount);
}

fn edge_detect(uv: vec2f, color: vec3f, mix_factor: f32) -> vec3f {
    let dims = params.resolution.xy;
    let edge_size = params.resolution.w;
    let texel = edge_size / dims;

    let tex = feedback_texture;
    let smp = source_sampler;
    let center = textureSample(tex, smp, uv).rgb;
    let right = textureSample(tex, smp, uv + vec2f(texel.x, 0.0)).rgb;
    let left = textureSample(tex, smp, uv - vec2f(texel.x, 0.0)).rgb;
    let up = textureSample(tex, smp, uv + vec2f(0.0, texel.y)).rgb;
    let down = textureSample(tex, smp, uv - vec2f(0.0, texel.y)).rgb;

    let dx = right - left;
    let dy = up - down;

    let edge_strength = clamp(length(dx) + length(dy), 0.0, 1.0);
    let edge_color = vec3f(edge_strength) * 0.9;

    return mix(color, edge_color, mix_factor);
}
