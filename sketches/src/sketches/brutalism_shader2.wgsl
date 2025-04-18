struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // edge_mix, edge_size, edge_thresh, geo_mix
    z: vec4f,

    // geo_size, geo_offs, contrast, brightness
    y: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_texture: texture_2d<f32>;

@group(1) @binding(1)
var source_sampler: sampler;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let sample = textureSample(source_texture, source_sampler, in.uv);
    var color = sample.rgb;

    let edge_mix = params.z.x;
    let geo_mix = params.z.w;
    let contrast = params.y.z;
    let brightness = params.y.w;

    color = edge_detect(in.uv, color, edge_mix);
    color = geometric_explode(in.uv, color, geo_mix);

    let stats = get_local_stats(in.uv);
    color = adaptive_enhance(color, stats, brightness, contrast);

    return vec4f(color, sample.a);
}

fn edge_detect(uv: vec2f, color: vec3f, mix_factor: f32) -> vec3f {
    let edge_size = params.z.y;
    let edge_thresh = params.z.z;

    let dims = textureDimensions(source_texture);
    let offset = vec2i(i32(edge_size), i32(edge_size));

    let right = textureLoad(
        source_texture, 
        vec2i(uv * vec2f(dims)) + vec2i(offset.x, 0),
        0
    );
    let left = textureLoad(
        source_texture, 
        vec2i(uv * vec2f(dims)) - vec2i(offset.x, 0),
        0
    );
    let top = textureLoad(
        source_texture, 
        vec2i(uv * vec2f(dims)) + vec2i(0, offset.y),
        0
    );
    let bottom = textureLoad(
        source_texture, 
        vec2i(uv * vec2f(dims)) - vec2i(0, offset.y),
        0
    );
    
    let edge_h = abs(right - left);
    let edge_v = abs(top - bottom);
    let edges = sqrt(edge_h * edge_h + edge_v * edge_v);
    
    let modified = vec3f(
        step(edge_thresh, edges.r),
        step(edge_thresh, edges.g),
        step(edge_thresh, edges.b)
    );
    
    return mix(color, modified, mix_factor);
}

fn geometric_explode(uv: vec2f, color: vec3f, mix_factor: f32) -> vec3f {
    let geo_size = params.y.x;  
    let geo_offs = params.y.y;

    let block_uv = floor(uv / geo_size);
    let offset = sin(dot(block_uv, vec2f(12.9898, 78.233))) * geo_offs;
    let center = vec2f(0.5, 0.5);
    let dir = normalize(uv - center);
    let displaced_uv = uv + dir * offset * mix_factor * 0.1;
    let displaced = textureSample(source_texture, source_sampler, displaced_uv);
    let block_color = displaced.rgb * (1.0 + offset * 0.);
    
    return mix(color, block_color, mix_factor);
}

fn adaptive_enhance(
    color: vec3f, 
    stats: vec2f, 
    brightness: f32, 
    contrast: f32
) -> vec3f {
    let luminance = dot(color, vec3f(0.299, 0.587, 0.114));
    let local_mean = stats.x;
    let local_variance = stats.y;

    let contrast_scale = 2.0;
    let local_diff = luminance - local_mean; 
    let contrasted = local_mean + 
        local_diff * (1.0 + contrast_scale * contrast);
    
    let final_lum = select(
        contrasted * (1.0 - abs(brightness)),
        contrasted + (1.0 - contrasted) * brightness,
        brightness >= 0.0
    );
    
    return color * (final_lum / luminance);
}

fn get_local_stats(uv: vec2f) -> vec2f {
    var local_mean = 0.0;
    var local_variance = 0.0;
    let pixel_offset = 1.0 / vec2f(textureDimensions(source_texture));
    
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let offset = vec2f(f32(i), f32(j)) * pixel_offset;
            let neighbor = 
                textureSample(source_texture, source_sampler, uv + offset).rgb;
            let lum = dot(neighbor, vec3f(0.299, 0.587, 0.114));
            local_mean += lum;
            local_variance += lum * lum;
        }
    }
    
    local_mean /= 9.0;
    local_variance = (local_variance / 9.0) - (local_mean * local_mean);
    return vec2f(local_mean, local_variance);
}