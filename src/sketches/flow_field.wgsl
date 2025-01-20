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

    // bg_alpha, bg_anim, ..unused
    a: vec4f,
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

    let pos_noise = rand_pcg(u32(frag.position.x + frag.position.y)) * bg_anim;
    let d = length(frag.position * 0.0005);

    if frag.vertex_type == VERTEX_TYPE_BG {
        let bg_base_color = vec3f(0.01);
        let color = mix(bg_base_color, chicago_flag_color(d), pos_noise);
        return vec4f(color, bg_alpha);
    } else {
        return vec4f(chicago_flag_color(d), 1.0);
    }
}

fn chicago_flag_color(d: f32) -> vec3f {
    let red = vec3f(1.0, 0.16, 0.16);
    let blue = vec3f(0.29, 0.56, 0.89);
    let white = vec3f(0.97);
    
    let color = mix(blue, red, smoothstep(0.1, 0.9, d)); 
    
    return mix(color, white, 0.0); 
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