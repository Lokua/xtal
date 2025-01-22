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

    // bg_alpha, bg_anim, slice_glitch, unused
    a: vec4f,

    // lightning, ...unused
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    let displace_mix = params.a.z;
    var out: VertexOutput;
    var p = vert.position;
    p = mix(p, displace(p), displace_mix);
    out.position = vec4f(p, 0.0, 1.0);
    out.vertex_type = vert.vertex_type;
    return out;
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4f {
    let bg_alpha = params.a.x;
    let bg_anim = params.a.y;
    let slice_glitch = params.a.w;
    let lightning = params.b.x * 5.0;

    let p = frag.position.xy * slice_glitch;
    let rain_direction = p.x + p.y;
    let pos_noise = rand_pcg(u32(rain_direction)) * bg_anim;
    let d = length(frag.position * 0.0005);

    var out_color: vec3f;
    var out_alpha: f32;

    if frag.vertex_type == VERTEX_TYPE_BG {
        let bg_base_color = vec3f(0.01);
        out_color =  mix(
            1.0 - bg_base_color, 
            gen_color(d), 
            pos_noise * lightning
        );
        out_alpha = bg_alpha;
    } else {
        out_color = gen_color(d);
        out_alpha = 1.0;
    }

    return vec4f(out_color, out_alpha);
}

fn gen_color(d: f32) -> vec3f {
    let a = vec3f(1.0, 0.16, 0.16);
    let b = vec3f(0.4, 0.56, 0.33);
    let white = vec3f(0.97);
    
    let color = mix(b, a, smoothstep(0., 0.9, d)); 
    
    return 1.0 - mix(color, white, 0.0); 
}

fn displace(p: vec2f) -> vec2f {
    let strength = 0.5;
    let p_scale = 133.0;
    let scale = 0.02;

    let scaled_coords = p * p_scale;
    let flow_x = sin(scaled_coords.x + scaled_coords.y);
    let flow_y = cos(scaled_coords.x - scaled_coords.y);
    let displacement = vec2f(flow_x, flow_y) * strength * scale;

    return p + displacement;
}

fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}