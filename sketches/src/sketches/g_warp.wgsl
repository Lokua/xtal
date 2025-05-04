struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, invert
    a: vec4f,
    // chroma, thickness, step_size, cell_size
    b: vec4f,
    // warp_amt, softness, a1, a2
    c: vec4f,
    // a3, ...
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
    let t = params.a.z * 0.25;
    let thickness = params.b.y;
    let cell_size = params.b.w;
    let warp_amt = params.c.x;
    let softness = params.c.y;
    let chroma = params.b.x;
    let invert = bool(params.a.w);
    let a1 = params.c.z;
    let a2 = params.c.w;
    let a3 = params.d.x;

    let p = correct_aspect(position) * 1.25;


    let dist = length(p);
    let cp = p / cell_size;

    let warped = 
        cos(cp) * 
        sin(dist + t) * 
        warp_amt;

    let d = length(abs(fract(warped) - 0.5));
    
    let line = 1.0 - smoothstep(
        thickness - softness, 
        thickness + softness, 
        dist + d
    );

    var o = vec3f(line) + (1.0 - mix(dist - d, dist + d, (a2 + a3) / 2.0));

    o = smoothstep(
        vec3f(a1, a2, a3) - 0.4, 
        vec3f(a1, a2, a3) + 0.4, 
        o
    );

    let steps = 4.0;
    o = o * floor(o * steps) / steps;

    o = rgb_to_oklch(o);
    o = vec3f(
        o.x * 0.63,
        o.y * chroma,
        o.z
    );

    o = oklch_to_rgb(o);
    return vec4f(select(o, 1.0 - o, invert), 1.0);

}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn rgb_to_oklch(rgb: vec3f) -> vec3f {
  let r = select(
    rgb.x / 12.92, 
    pow((rgb.x + 0.055) / 1.055, 2.4), 
    rgb.x > 0.04045
  );
  let g = select(
    rgb.y / 12.92, 
    pow((rgb.y + 0.055) / 1.055, 2.4), 
    rgb.y > 0.04045
  );
  let b = select(
    rgb.z / 12.92, 
    pow((rgb.z + 0.055) / 1.055, 2.4), 
    rgb.z > 0.04045
  );

  let l = 0.41222147 * r + 0.53633254 * g + 0.05144599 * b;
  let m = 0.21190350 * r + 0.68069954 * g + 0.10739696 * b;
  let s = 0.08830246 * r + 0.28171884 * g + 0.62997870 * b;

  let l_ = pow(l, 1.0 / 3.0);
  let m_ = pow(m, 1.0 / 3.0);
  let s_ = pow(s, 1.0 / 3.0);

  let ok_l = 0.21045426 * l_ + 0.79361779 * m_ - 0.00407205 * s_;
  let ok_a = 1.97799850 * l_ - 2.42859220 * m_ + 0.45059371 * s_;
  let ok_b = 0.02590404 * l_ + 0.78277177 * m_ - 0.80867577 * s_;

  let c = length(vec2f(ok_a, ok_b));
  let h = fract(degrees(atan2(ok_b, ok_a)) / 360.0);

  return vec3f(ok_l, c, h);
}

fn oklch_to_rgb(oklch: vec3f) -> vec3f {
  let l = oklch.x;
  let c = oklch.y;
  let h = oklch.z * 360.0;

  let cx = cos(radians(h)) * c;
  let cy = sin(radians(h)) * c;

  let l_ = l + 0.39633778 * cx + 0.21580376 * cy;
  let m_ = l - 0.10556135 * cx - 0.06385417 * cy;
  let s_ = l - 0.08948418 * cx - 1.29148555 * cy;

  let l3 = l_ * l_ * l_;
  let m3 = m_ * m_ * m_;
  let s3 = s_ * s_ * s_;

  let r_lin = 4.07674166 * l3 - 3.30771159 * m3 + 0.23096993 * s3;
  let g_lin = -1.26843800 * l3 + 2.60975740 * m3 - 0.34131940 * s3;
  let b_lin = -0.00419609 * l3 - 0.70341861 * m3 + 1.70761470 * s3;

  let r = select(
    12.92 * r_lin, 
    1.055 * pow(r_lin, 1.0 / 2.4) - 0.055, 
    r_lin > 0.0031308
  );
  let g = select(
    12.92 * g_lin, 
    1.055 * pow(g_lin, 1.0 / 2.4) - 0.055, 
    g_lin > 0.0031308
  );
  let b = select(
    12.92 * b_lin, 
    1.055 * pow(b_lin, 1.0 / 2.4) - 0.055, 
    b_lin > 0.0031308
  );

  return clamp(vec3f(r, g, b), vec3f(0.0), vec3f(1.0));
}