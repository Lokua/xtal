const TAU: f32 = 6.283185307;
const MAX_STRESS: i32 = 4096;

struct Params {
    // w, h, beats, sweep_x
    a: vec4f,
    // stres, unused, unused, unused
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

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let aspect = resolution.x / resolution.y;
    let beats = params.a.z;
    let sweep_x = clamp(params.a.w, 0.0, 1.0);
    let stres = params.b.x;
    let stress_iters = i32(clamp(stres, 1.0, f32(MAX_STRESS)));
    let phase = fract(beats);
    let tri = 1.0 - abs(phase * 2.0 - 1.0);
    let transport_x = 0.08 + tri * 0.84;

    let bg = vec3f(0.035, 0.04, 0.05);
    var color = bg;

    // Phase anchors for a 1-beat triangle:
    // phase 0/1 -> x=0.08, phase 0.5 -> x=0.92, midpoint -> x=0.50.
    let anchor_l = 1.0 - smoothstep(0.0, 0.0035, abs((in.uv.x - 0.08) * aspect));
    let anchor_m = 1.0 - smoothstep(0.0, 0.0035, abs((in.uv.x - 0.50) * aspect));
    let anchor_r = 1.0 - smoothstep(0.0, 0.0035, abs((in.uv.x - 0.92) * aspect));
    color = mix(color, vec3f(0.11, 0.12, 0.14), (anchor_l + anchor_m + anchor_r) * 0.35);

    // Two lanes, same target motion:
    // top = triangle from raw transport phase
    // bottom = YAML triangle control output
    let lane_transport = 1.0 - smoothstep(0.0, 0.0035, abs(in.uv.y - 0.66));
    let lane_yaml = 1.0 - smoothstep(0.0, 0.0035, abs(in.uv.y - 0.34));
    color = mix(color, vec3f(0.12, 0.14, 0.16), (lane_transport + lane_yaml) * 0.5);

    // Transport playhead from raw beats phase (triangle mapped).
    let transport_line = 1.0
        - smoothstep(
            0.0,
            0.006,
            abs((in.uv.x - transport_x) * aspect),
        );
    let transport_band = 1.0 - smoothstep(0.0, 0.06, abs(in.uv.y - 0.66));
    color = mix(color, vec3f(0.28, 0.82, 1.0), transport_line * transport_band);

    // YAML triangle playhead (should overlap transport lane if timing path is correct).
    let yaml_line = 1.0
        - smoothstep(
            0.0,
            0.006,
            abs((in.uv.x - sweep_x) * aspect),
        );
    let yaml_band = 1.0 - smoothstep(0.0, 0.06, abs(in.uv.y - 0.34));
    color = mix(color, vec3f(0.98, 0.33, 0.12), yaml_line * yaml_band);

    // Marker dots for both lanes.
    let dot_transport_pos = vec2f(transport_x, 0.66);
    let dot_transport_dist = length(vec2f(
        (in.uv.x - dot_transport_pos.x) * aspect,
        in.uv.y - dot_transport_pos.y,
    ));
    let dot_transport = 1.0 - smoothstep(0.0, 0.017, dot_transport_dist);
    color = mix(color, vec3f(0.88, 0.98, 1.0), dot_transport);

    let dot_yaml_pos = vec2f(sweep_x, 0.34);
    let dot_yaml_dist = length(vec2f(
        (in.uv.x - dot_yaml_pos.x) * aspect,
        in.uv.y - dot_yaml_pos.y,
    ));
    let dot_yaml = 1.0 - smoothstep(0.0, 0.017, dot_yaml_dist);
    color = mix(color, vec3f(1.0, 0.93, 0.75), dot_yaml);

    // Beat pulse indicator in the top-left corner with long hold,
    // so it remains visible even when FPS is low.
    let pulse = 1.0 - smoothstep(0.0, 0.35, phase);
    let pulse_box = (1.0 - smoothstep(0.02, 0.023, abs(in.uv.x - 0.04)))
        * (1.0 - smoothstep(0.04, 0.043, abs(in.uv.y - 0.94)));
    color = mix(color, vec3f(0.92, 0.96, 1.0), pulse_box * pulse);

    // Hidden stress load: heavy math path with subtle visual influence.
    var sink = 0.0;
    for (var i = 0; i < MAX_STRESS; i++) {
        if i >= stress_iters {
            break;
        }

        let fi = f32(i);
        let p = vec2f(
            in.uv.x * (1.0 + fi * 0.0008),
            in.uv.y * (1.0 + fi * 0.0007),
        );
        sink += sin((p.x + p.y) * TAU + beats * 0.7 + fi * 0.015)
            * cos((p.x - p.y) * TAU * 1.3 - beats * 0.4 + fi * 0.011);
    }

    let stress_tint = fract(abs(sink) * 0.00002);
    color += vec3f(0.004, 0.006, 0.008) * stress_tint;

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
