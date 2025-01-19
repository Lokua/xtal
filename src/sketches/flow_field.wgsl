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

    // bg_alpha, ...unused
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

    if (frag.vertex_type == VERTEX_TYPE_BG) {
        return vec4f(vec3f(1.0), bg_alpha);
    }

    return vec4f(0.2, 0.0, 0.3, 1.0);
}