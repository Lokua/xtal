struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var video_sampler: sampler;

@group(1) @binding(1)
var video_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let uv = vec2f(in.uv.x, 1.0 - in.uv.y);
    let slice_uv = apply_slice_shift(uv);
    let zoom_uv = apply_zoom(slice_uv);
    let color = textureSample(video_tex, video_sampler, zoom_uv);
    return vec4f(apply_video_fx(color.rgb), 1.0);
}

fn apply_zoom(uv: vec2f) -> vec2f {
    let amount = clamp(params.c.x, 0.0, 1.0);
    let manual_offset = params.c.yz;
    let auto_offset = params.d.xy;
    var offset = manual_offset;

    if (params.c.w > 0.5) {
        offset = auto_offset;
    }

    offset = clamp(offset, vec2f(-1.0), vec2f(1.0));
    let scale = mix(1.0, 4.0, amount);
    let center = vec2f(0.5, 0.5) + offset * 0.5;
    return (uv - center) / scale + center;
}

fn apply_slice_shift(uv: vec2f) -> vec2f {
    let amount = clamp(params.e.x, 0.0, 1.0);
    let slices = max(floor(params.e.y), 1.0);
    let offset = clamp(params.e.z, -0.5, 0.5);
    let use_vertical = step(0.5, params.e.w);
    let boundary = i32(params.f.x + 0.5);
    let coord = mix(uv.y, uv.x, use_vertical);
    let slice_index = floor(coord * slices);
    let alternating = select(-1.0, 1.0, (i32(slice_index) & 1) == 0);
    let shift = alternating * offset * amount;
    let shifted = uv
        + mix(vec2f(shift, 0.0), vec2f(0.0, shift), use_vertical);
    return apply_slice_boundary(shifted, boundary);
}

fn apply_slice_boundary(uv: vec2f, boundary: i32) -> vec2f {
    if (boundary == 1) {
        return fract(uv);
    }
    if (boundary == 2) {
        return 1.0 - abs(fract(uv * 0.5) * 2.0 - 1.0);
    }
    return clamp(uv, vec2f(0.0), vec2f(1.0));
}

fn apply_video_fx(color: vec3f) -> vec3f {
    let black_white = clamp(params.b.x, 0.0, 1.0);
    let invert = clamp(params.b.y, 0.0, 1.0);
    let posterize = clamp(params.b.z, 0.0, 1.0);
    let posterize_levels = clamp(params.b.w, 2.0, 32.0);
    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    var out_color = mix(color, vec3f(luma), black_white);
    out_color = mix(out_color, 1.0 - out_color, invert);
    out_color = mix(
        out_color,
        posterize_color(out_color, posterize_levels),
        posterize,
    );
    return out_color;
}

fn posterize_color(color: vec3f, levels: f32) -> vec3f {
    return floor(color * levels) / max(levels - 1.0, 1.0);
}
