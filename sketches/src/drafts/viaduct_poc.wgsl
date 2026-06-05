struct Params {
    a: vec4f,
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var video_sampler: sampler;

@group(1) @binding(1)
var video_0_tex: texture_2d<f32>;

@group(1) @binding(2)
var video_1_tex: texture_2d<f32>;

@group(1) @binding(3)
var video_2_tex: texture_2d<f32>;

@group(1) @binding(4)
var video_3_tex: texture_2d<f32>;

@group(1) @binding(5)
var video_4_tex: texture_2d<f32>;

@group(1) @binding(6)
var video_5_tex: texture_2d<f32>;

@group(1) @binding(7)
var video_6_tex: texture_2d<f32>;

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
    let loop_duration = selected_loop_duration(i32(params.b.y + 0.5));
    let active_video = i32(floor(params.a.z / loop_duration)) % 7;
    return sample_active_video(uv, screen_size, fit_mode, active_video);
}

fn selected_loop_duration(index: i32) -> f32 {
    if (index == 0) {
        return 1.0;
    }
    if (index == 1) {
        return 2.0;
    }
    if (index == 2) {
        return 4.0;
    }
    if (index == 3) {
        return 8.0;
    }
    if (index == 4) {
        return 12.0;
    }
    return 16.0;
}

fn sample_active_video(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
    active_video: i32,
) -> vec4f {
    if (active_video == 1) {
        return sample_video_1(uv, screen_size, fit_mode);
    }
    if (active_video == 2) {
        return sample_video_2(uv, screen_size, fit_mode);
    }
    if (active_video == 3) {
        return sample_video_3(uv, screen_size, fit_mode);
    }
    if (active_video == 4) {
        return sample_video_4(uv, screen_size, fit_mode);
    }
    if (active_video == 5) {
        return sample_video_5(uv, screen_size, fit_mode);
    }
    if (active_video == 6) {
        return sample_video_6(uv, screen_size, fit_mode);
    }
    return sample_video_0(uv, screen_size, fit_mode);
}

fn sample_video_0(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_0_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_0_tex, video_sampler, fit_uv);
}

fn sample_video_1(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_1_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_1_tex, video_sampler, fit_uv);
}

fn sample_video_2(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_2_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_2_tex, video_sampler, fit_uv);
}

fn sample_video_3(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_3_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_3_tex, video_sampler, fit_uv);
}

fn sample_video_4(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_4_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_4_tex, video_sampler, fit_uv);
}

fn sample_video_5(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_5_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_5_tex, video_sampler, fit_uv);
}

fn sample_video_6(
    uv: vec2f,
    screen_size: vec2f,
    fit_mode: i32,
) -> vec4f {
    let video_size = vec2f(textureDimensions(video_6_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);
    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(video_6_tex, video_sampler, fit_uv);
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
