const PI: f32 = 3.14159265359;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // invert, smooth_mix, time, time_2
    a: vec4f,

    // t1, t2, t3, post_mix
    b: vec4f,

    // r1, r2, r3, unused
    c: vec4f,
    
    // g1, g2, g3, unused
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let contrast = params.a.x;
    let time = params.a.z;
    let post_mix = params.b.w;
    let t1 = params.b.x;
    let t2 = params.b.y;
    let t3 = params.b.z;
    let r1 = params.c.x;
    let r2 = params.c.y;
    let r3 = params.c.z;
    let g1 = params.d.x;
    let g2 = params.d.y;
    let g3 = params.d.z;

    let aspect = params.resolution.x / params.resolution.y;
    var p = position;
    p.x *= aspect;

    var grid_1 = create_grid(p, g1, t1, r1);
    var grid_2 = create_grid(p, g2, t2, r2);
    let grid_3 = create_grid(p, g3, t3, r3);

    grid_1.r = 0.25;
    grid_2.b = 0.90;

    let color = mix(mix(grid_1, grid_2, 0.5), grid_3, 0.33);
    let adjusted_color = 0.5 + (color - 0.5) * contrast;

    var c = vec3f(adjusted_color);
    c = film_grain(c, p, 1.0);
    c = glitch_blocks(c, p, 4.0, post_mix);

    return vec4f(c, 1.0);
}

fn create_grid(
    pos: vec2f,
    resolution: f32,
    contour_interval: f32,
    radius: f32
) -> vec3f {
    let invert = params.a.x;
    let smooth_mix = params.a.y;
    let time = params.a.z;
    let time_2 = params.a.w;

    let p = pos * resolution;
    let cell = fract(p) - 0.5;
    let grid_coord = floor(p);

    let x_wave = 1.0 - sin(grid_coord.x);
    let x_warp = tanh(grid_coord.x);
    let x_pattern = mix(x_wave, x_warp, time);
    let y_wave = cos(grid_coord.y);
    let y_warp = 1.0 - tanh(grid_coord.y);
    let y_pattern = mix(y_wave, y_warp, time_2);
    let pattern = x_pattern * y_pattern;

    let d = length(cell) * contour_interval + pattern;

    let invert_mix = random2(grid_coord);

    let normal_color = mix(
        vec3f(smoothstep(0.0, radius, d)),
        vec3f(step(d, radius)),
        smooth_mix
    );

    let inverted_color = mix(
        vec3f(1.0 - smoothstep(0.0, radius, d)),
        vec3f(1.0 - step(d, radius)),
        smooth_mix
    );

    return mix(normal_color, inverted_color, invert_mix);
}

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = random2(p);
    return clamp(color + (random - 0.5) * intensity, vec3f(0.0), vec3f(1.0));
}

fn glitch_blocks(color: vec3f, p: vec2f, block_size: f32, intensity: f32) -> vec3f {
    let block = floor(p * block_size);
    let noise = fract(sin(dot(block, vec2f(12.9898, 78.233))) * 43758.5453);
    return mix(color, vec3f(1.0) - color, step(1.0 - intensity, noise));
}

fn random2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}
