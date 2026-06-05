struct Params {
    a: vec4f,
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var video_sampler: sampler;

@group(1) @binding(1)
var video_a_tex: texture_2d<f32>;

@group(1) @binding(2)
var video_b_tex: texture_2d<f32>;

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
    let screen_size = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let fit_mode = i32(params.b.x + 0.5);
    let xfade = clamp(params.a.w, 0.0, 1.0);
    let video_a = sample_video_a(uv, screen_size, fit_mode);
    let video_b = sample_video_b(uv, screen_size, fit_mode);
    let color = mix(video_a.rgb, video_b.rgb, xfade);
    return vec4f(color, 1.0);
}

fn sample_video_a(uv: vec2f, screen_size: vec2f, fit_mode: i32) -> vec4f {
    let video_size = vec2f(textureDimensions(video_a_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_a_tex, video_sampler, fit_uv);
}

fn sample_video_b(uv: vec2f, screen_size: vec2f, fit_mode: i32) -> vec4f {
    let video_size = vec2f(textureDimensions(video_b_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_b_tex, video_sampler, fit_uv);
}

fn fit_video_uv(
    uv: vec2f,
    screen_size: vec2f,
    video_size: vec2f,
    fit_mode: i32,
) -> vec2f {
    if (fit_mode == 2) {
        return uv;
    }

    let screen = max(screen_size, vec2f(1.0, 1.0));
    let video = max(video_size, vec2f(1.0, 1.0));
    let screen_aspect = screen.x / screen.y;
    let video_aspect = video.x / video.y;
    var scale = vec2f(1.0, 1.0);

    if (fit_mode == 1) {
        if (screen_aspect > video_aspect) {
            scale = vec2f(video_aspect / screen_aspect, 1.0);
        } else {
            scale = vec2f(1.0, screen_aspect / video_aspect);
        }
    } else {
        if (screen_aspect > video_aspect) {
            scale = vec2f(1.0, screen_aspect / video_aspect);
        } else {
            scale = vec2f(video_aspect / screen_aspect, 1.0);
        }
    }

    return (uv - vec2f(0.5, 0.5)) / scale + vec2f(0.5, 0.5);
}

fn is_outside(uv: vec2f) -> bool {
    return uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0;
}
