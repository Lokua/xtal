// Forked from https://www.shadertoy.com/view/WX23Dz

const EPSILON: f32 = 1.1920929e-7;
const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, direction
    a: vec4f,
    // h1, s1, v1, h2
    b: vec4f,
    // s2, v2, detail, ray_scale_factor
    c: vec4f,
    // bg_alpha, t_mult, depth_scale, UNUSED
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
fn fs_main(@location(0) pos: vec2f) -> @location(0) vec4f {
    let bg_alpha = params.d.x;
    let c0 = cloud_tunnel(pos);
    return vec4f(c0, bg_alpha);
}

fn cloud_tunnel(pos: vec2f) -> vec3f {
    let t_mult = params.d.y;
    let t = params.a.z * t_mult;
    let direction = select(-1.0, 1.0, params.a.w == 0.0);
    let h1 = params.b.x;
    let s1 = params.b.y;
    let v1 = params.b.z;
    let h2 = params.b.w;
    let s2 = params.c.x;
    let v2 = params.c.y;
    let detail = params.c.z;
    let ray_scale_factor = params.c.w;
    let depth_scale = params.d.z;

    // step size
    var step = 0.02;
    var i = 0.0;
    // distance accumulator
    var d = 0.0;
    // noise value
    var n: f32;
    // output color
    var o = vec3f(0.0);

    var aspect_pos = correct_aspect(pos);
    
    // ray position
    var p = vec3f(sin(t * 0.25) * 0.333, cos(t) * 0.125, -10.0);

    // in HSV
    let color1 = vec3f(h1, s1, v1);
    let color2 = vec3f(h2, s2, v2);

    loop {
        i += 1.0;
        
        if i > 60.0 || step <= 0.01 {
            break;
        }
    
        n = length(p.xy) - 1.0;
        step = 1.5 - length(p.xy) - n * 0.3;

        var inner_n = 0.075;
        while inner_n < 2.0 {
            step -= abs(dot(sin(p * inner_n * 65.0), vec3f(detail))) / inner_n;
            let divisor = 2.0;
            let inner = ((inner_n + inner_n) + (inner_n * 1.4142)) / divisor;
            inner_n = max(inner_n * 1.05, inner);
        }

        let cloud_depth = smoothstep(0.0, 20.0, d);
        let center_dist = length(p.xy);
        let center_factor = smoothstep(0.0, 10.5, center_dist);
        o += mix(color1, color2, cloud_depth) * center_factor;

        let depth_rotation = sin(p.z * 0.02) * depth_scale;
        let sin_rot = sin(depth_rotation);
        let cos_rot = cos(depth_rotation * 0.333);
        let rotated_pos = vec2f(
            aspect_pos.x * cos_rot - aspect_pos.y * sin_rot,
            aspect_pos.x * sin_rot + aspect_pos.y * cos_rot
        );

        p += (vec3f(rotated_pos, 1.0) * ray_scale_factor * step) * direction;
        d += step;
    }

    o = hsv_to_rgb(o);

    if d > 100.0 {
        o = vec3f(1.0);
    } else {
        o = 1.0 - o;
    }

    o = pow(o.rgb, vec3f(2.0));
    o = vec3f(1.0) - o;

    return o;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;
    
    if (s == 0.0) {
        return vec3f(v, v, v);
    }
    
    let i = floor(h * 6.0);
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    
    var r = 0.0;
    var g = 0.0;
    var b = 0.0;
    
    if (i % 6.0 == 0.0) {
        r = v; g = t; b = p;
    } else if (i % 6.0 == 1.0) {
        r = q; g = v; b = p;
    } else if (i % 6.0 == 2.0) {
        r = p; g = v; b = t;
    } else if (i % 6.0 == 3.0) {
        r = p; g = q; b = v;
    } else if (i % 6.0 == 4.0) {
        r = t; g = p; b = v;
    } else {
        r = v; g = p; b = q;
    }
    
    return vec3f(r, g, b);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn rotate_point(p: vec2f, angle_degrees: f32) -> vec2f {
    let angle = radians(angle_degrees);
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    
    return vec2f(
        p.x * cos_angle - p.y * sin_angle,
        p.x * sin_angle + p.y * cos_angle
    );
}