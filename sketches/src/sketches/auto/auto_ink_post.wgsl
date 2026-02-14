struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
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

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var source_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let w = params.a.x;
    let h = params.a.y;
    let texel = vec2f(1.0 / w, 1.0 / h);
    let frag = in.uv * vec2f(w, h);

    let bleed_amount = params.g.x;
    let bleed_px = params.g.y;
    let edge_threshold = params.g.z;
    let edge_softness = params.g.w;
    let distress_amount = params.h.x;
    let speck_density = params.h.y;
    let dropout = params.h.z;
    let misregister_px = params.h.w;

    let center = textureSample(source_texture, source_sampler, in.uv);
    let left = textureSample(
        source_texture,
        source_sampler,
        in.uv - vec2f(texel.x, 0.0),
    );
    let right = textureSample(
        source_texture,
        source_sampler,
        in.uv + vec2f(texel.x, 0.0),
    );
    let down = textureSample(
        source_texture,
        source_sampler,
        in.uv - vec2f(0.0, texel.y),
    );
    let up = textureSample(
        source_texture,
        source_sampler,
        in.uv + vec2f(0.0, texel.y),
    );

    let lum_center = luminance(center.rgb);
    let lum_l = luminance(left.rgb);
    let lum_r = luminance(right.rgb);
    let lum_d = luminance(down.rgb);
    let lum_u = luminance(up.rgb);

    let dx = lum_r - lum_l;
    let dy = lum_u - lum_d;
    let edge = abs(dx) + abs(dy);
    let edge_mask = smoothstep(
        edge_threshold,
        edge_threshold + edge_softness,
        edge,
    );

    let n = normalize(vec2f(dx, dy) + vec2f(1e-5, 0.0));
    let offset = n * texel * bleed_px * edge_mask;

    let r = textureSample(
        source_texture,
        source_sampler,
        in.uv + offset,
    ).r;
    let g = center.g;
    let b = textureSample(
        source_texture,
        source_sampler,
        in.uv - offset,
    ).b;
    let split = vec3f(r, g, b);

    let amt = clamp(bleed_amount * edge_mask, 0.0, 1.0);
    var color = mix(center.rgb, split, amt);

    let mis_cell = floor(frag * 0.08);
    let mis_x = hash21(mis_cell + vec2f(2.1, 7.4)) - 0.5;
    let mis_y = hash21(mis_cell + vec2f(5.9, 1.6)) - 0.5;
    let mis_uv = vec2f(mis_x, mis_y) * texel * misregister_px
        * distress_amount;

    let mis_r = textureSample(
        source_texture,
        source_sampler,
        in.uv + mis_uv,
    ).r;
    let mis_b = textureSample(
        source_texture,
        source_sampler,
        in.uv - mis_uv,
    ).b;
    color.r = mix(color.r, mis_r, distress_amount);
    color.b = mix(color.b, mis_b, distress_amount);

    let speck_pick = hash21(frag * 0.91 + vec2f(11.7, 3.3));
    let speck_thr = 1.0 - speck_density * distress_amount * 0.22;
    let speck = step(speck_thr, speck_pick);
    color = mix(color, vec3f(0.06), speck * distress_amount * 0.7);

    let drop_pick = hash21(floor(frag * 0.11) + vec2f(17.2, 2.6));
    let drop_thr = 1.0 - dropout * distress_amount;
    let drop = step(drop_thr, drop_pick);
    let paper = vec3f(0.985, 0.975, 0.955);
    color = mix(color, paper, drop * distress_amount * 0.75);

    return vec4f(color, center.a);
}

fn luminance(color: vec3f) -> f32 {
    return dot(color, vec3f(0.299, 0.587, 0.114));
}

fn hash21(p: vec2f) -> f32 {
    var q = fract(vec3f(p.xyx) * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}
