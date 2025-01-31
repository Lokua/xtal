const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;
const DEBUG: bool = false;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) center: vec3f,
    @location(2) @interpolate(flat) layer: f32
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) pos: vec3f,
    @location(1) local_pos: vec3f,
    @location(2) @interpolate(flat) layer: f32
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // rot_x, rot_y, rot_z, z_offset
    a: vec4f,

    // scale, ...unused 
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.layer = vert.layer;
    out.local_pos = vert.position;

    if vert.layer < FOREGROUND {
        let p = correct_aspect(vert.position);
        out.clip_position = vec4f(p.xy, 0.999, 1.0);
        out.pos = vec3f(p.xy, 0.999);

        return out;
    }

    // TRS = Translate, Rotate, Scale 
    // (applied in reverse, because...that's what you do?)
    let r_x = params.a.x;
    let r_y = params.a.y;
    let r_z = params.a.z;
    let z_offset = clamp(params.a.w, -10.0, -0.5);
    let scale = params.b.x;

    let scaled_position = vert.position * scale;

    var rotated = rotate_x(scaled_position, r_x);
    rotated = rotate_y(rotated, r_y);
    rotated = rotate_z(rotated, r_z);
    
    let translated = vec3f(rotated.x, rotated.y, rotated.z + z_offset) + 
        vert.center;

    // Perspective projection matrix
    // Field of view
    let fov = radians(45.0);
    let aspect = params.resolution.x / params.resolution.y; 
    let near = 0.1;
    let far = 100.0;

    let f = 1.0 / tan(fov / 2.0);
    let range_inv = 1.0 / (near - far);

    let proj = mat4x4f(
        vec4f(f / aspect, 0.0, 0.0, 0.0),
        vec4f(0.0, f, 0.0, 0.0),
        vec4f(0.0, 0.0, far * range_inv, -1.0),
        vec4f(0.0, 0.0, near * far * range_inv, 0.0)
    );

    out.clip_position = proj * vec4f(translated, 1.0);
    out.pos = translated;

    return out;
}

@fragment
fn fs_main(vout: VertexOutput) -> @location(0) vec4f {
    if DEBUG {
        return vec4f(vout.layer, vout.layer, vout.layer, 1.0);
    }

    if vout.layer < FOREGROUND { 
        return vec4f(1.0);
    } 
    
    let pos = vout.local_pos;
    let light_dir = normalize(vec3f(0.5, 0.7, 0.5));
    
    var normal = vec3f(0.0);
    let eps = 0.01;
    if abs(abs(pos.x) - 0.5) < eps {
        normal = vec3f(sign(pos.x), 0.0, 0.0);
    } else if abs(abs(pos.y) - 0.5) < eps {
        normal = vec3f(0.0, sign(pos.y), 0.0);
    } else {
        normal = vec3f(0.0, 0.0, sign(pos.z));
    }
    
    let ambient = 0.6;
    let diffuse = max(dot(normal, light_dir), 0.0);
    let light = ambient + diffuse * (1.0 - ambient); 
    
    let face_color = vec3f(
        1.0 - abs(normal.x) * 0.01,
        1.0 - abs(normal.y) * 0.01,
        1.0 - abs(normal.z) * 0.01, 
    );

    let color = face_color * light;
    return vec4f(color, 1.0);
}

fn correct_aspect(position: vec3f) -> vec3f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    return vec3f(position.x * aspect, position.y, position.z);
}

fn rotate_x(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x,
        p.y * c - p.z * s,
        p.y * s + p.z * c
    );
}

fn rotate_y(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.z * s,
        p.y,
        p.x * s + p.z * c
    );
}

fn rotate_z(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.y * s,
        p.x * s + p.y * c,
        p.z
    );
}