struct VertexInput {
    @location(0) position: vec2f,
    @location(1) color: vec4f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(1) color: vec4f
};

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.color = vert.color;
    return out;
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4f {
    return frag.color; // Use interpolated color
}