struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

fn circle_mask(uv: vec2f, center: vec2f, radius: f32, aspect: f32) -> f32 {
    let p = vec2f((uv.x - center.x) * aspect, uv.y - center.y);
    let dist = length(p);
    return 1.0 - smoothstep(radius, radius + 0.003, dist);
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
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let aspect = resolution.x / resolution.y;
    let radius = 0.025;

    let top_x = clamp(params.a.w, 0.0, 1.0);
    let mid_x = clamp(params.b.x, 0.0, 1.0);
    let bottom_x = clamp(params.c.x, 0.0, 1.0);
    let random_x = clamp(params.d.x, 0.0, 1.0);
    let automate_x = clamp(params.d.y, 0.0, 1.0);

    let top_center = vec2f(top_x, 0.875);
    let mid_center = vec2f(mid_x, 0.625);
    let bottom_center = vec2f(bottom_x, 0.375);
    let random_center = vec2f(random_x, 0.225);
    let automate_center = vec2f(automate_x, 0.075);

    let top_mask = circle_mask(in.uv, top_center, radius, aspect);
    let mid_mask = circle_mask(in.uv, mid_center, radius, aspect);
    let bottom_mask = circle_mask(in.uv, bottom_center, radius, aspect);
    let random_mask = circle_mask(in.uv, random_center, radius, aspect);
    let automate_mask = circle_mask(in.uv, automate_center, radius, aspect);

    var color = vec3f(1.0);
    color = mix(color, vec3f(0.0), top_mask);
    color = mix(color, vec3f(1.0, 0.0, 0.0), mid_mask);
    color = mix(color, vec3f(0.0, 1.0, 0.0), bottom_mask);
    color = mix(color, vec3f(0.0, 0.35, 1.0), random_mask);
    color = mix(color, vec3f(1.0, 0.55, 0.0), automate_mask);

    return vec4f(color, 1.0);
}
