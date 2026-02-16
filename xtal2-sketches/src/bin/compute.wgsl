struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var field: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let dim = textureDimensions(field);

    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }

    let uv = vec2f(gid.xy) / vec2f(dim);
    let p = uv * 2.0 - vec2f(1.0, 1.0);

    let t = params.a.z;
    let freq = max(0.2, params.a.w);
    let warp = max(0.0, params.b.x);

    let ang = atan2(p.y, p.x);
    let rad = length(p);

    let wave_a = sin((p.x + p.y * 0.2) * 12.0 * freq + t * 1.5);
    let wave_b = cos((ang * 6.0 + rad * 14.0) + t * (0.5 + warp));

    let v = 0.5 + 0.5 * (wave_a * 0.6 + wave_b * 0.4);

    let color = vec3f(v, 0.35 + 0.65 * v, 1.0 - v * 0.8);
    textureStore(field, vec2i(gid.xy), vec4f(color, 1.0));
}
