const VERTEX_TYPE_BG = 0.0;
const VERTEX_TYPE_AGENT = 1.0;

struct VertexInput {
    @location(0) position: vec2f,
    @location(1) vertex_type: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(1) vertex_type: f32
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // bg_alpha, bg_anim, flim_grain, glitch_size
    a: vec4f,

    // glitch_amount, ...unused
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.vertex_type = vert.vertex_type;
    return out;
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4f {
    let bg_alpha = params.a.x;
    let bg_anim = params.a.y;
    let grain_amount = params.a.z;
    let glitch_size = params.a.w;
    let glitch_amount = params.b.x;

    let p = frag.position.xy;
    let pos_noise = rand_pcg(u32(p.x + p.y)) * bg_anim;
    let d = length(frag.position * 0.0005);

    var out_color: vec3f;
    var out_alpha: f32;

    if frag.vertex_type == VERTEX_TYPE_BG {
        let bg_base_color = vec3f(0.01);
        out_color = mix(bg_base_color, chicago_flag_color(d), pos_noise);
        out_alpha = bg_alpha;
    } else {
        out_color = chicago_flag_color(d);
        out_alpha = 1.0;
    }

    let grained = film_grain(out_color, p, grain_amount);
    let glitched = glitch_blocks(grained, p, glitch_size, glitch_amount);
    return vec4f(glitched, out_alpha);
}

fn chicago_flag_color(d: f32) -> vec3f {
    let red = vec3f(1.0, 0.16, 0.16);
    let blue = vec3f(0.29, 0.56, 0.89);
    let white = vec3f(0.97);
    
    let color = mix(blue, red, smoothstep(0.1, 0.9, d)); 
    
    return mix(color, white, 0.0); 
}

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = random_v2(p);
    return clamp(color + (random - 0.5) * intensity, vec3f(0.0), vec3f(1.0));
}

fn glitch_blocks(
    color: vec3f, 
    p: vec2f, 
    block_size: f32, 
    intensity: f32
) -> vec3f {
    let block = floor(p * block_size);
    let noise = random_v2(block);
    return mix(color, vec3f(1.0) - color, step(1.0 - intensity, noise));
}

// Basic random number generation (PCG)
fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

fn random_v2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}