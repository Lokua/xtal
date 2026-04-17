# xtal GPU Rendering: A Deep-Dive Tutorial

> **Who this is for:** Joshua (the author), and anyone new to wgpu who wants to
> understand how a real-world creative-coding renderer is structured. We assume
> basic Rust familiarity but no prior GPU knowledge.

---

## Table of Contents

1. [What is wgpu and why does any of this matter?](#1-what-is-wgpu-and-why-does-any-of-this-matter)
2. [The Big Picture: How a Frame Gets to Your Screen](#2-the-big-picture-how-a-frame-gets-to-your-screen)
3. [The Render Graph: Declaring What You Want](#3-the-render-graph-declaring-what-you-want)
4. [Resources: Textures, Uniforms, and Images](#4-resources-textures-uniforms-and-images)
5. [Nodes: The Building Blocks of a Graph](#5-nodes-the-building-blocks-of-a-graph)
6. [Compiling the Graph: From Spec to GPU Objects](#6-compiling-the-graph-from-spec-to-gpu-objects)
7. [Uniforms: Talking to Your Shaders](#7-uniforms-talking-to-your-shaders)
8. [Bind Groups: The GPU's Way of Passing Data](#8-bind-groups-the-gpus-way-of-passing-data)
9. [Shaders: Vertex, Fragment, and Compute](#9-shaders-vertex-fragment-and-compute)
10. [The Frame Loop: Executing the Graph Every Frame](#10-the-frame-loop-executing-the-graph-every-frame)
11. [Offscreen Textures and the Render Target System](#11-offscreen-textures-and-the-render-target-system)
12. [Ping-Pong and Feedback Effects](#12-ping-pong-and-feedback-effects)
13. [Compute Shaders: Direct Texture Writing](#13-compute-shaders-direct-texture-writing)
14. [The Present Blit: Getting Offscreen Work onto the Screen](#14-the-present-blit-getting-offscreen-work-onto-the-screen)
15. [Shader Hot Reload](#15-shader-hot-reload)
16. [Row Padding and Memory Alignment](#16-row-padding-and-memory-alignment)
17. [Putting It All Together: Walkthrough of a Multi-Pass Sketch](#17-putting-it-all-together-walkthrough-of-a-multi-pass-sketch)

---

## 1. What is wgpu and why does any of this matter?

wgpu is a Rust library that talks to your GPU. It's a safe, cross-platform
abstraction over the three major modern graphics APIs: Vulkan (Linux/Windows),
Metal (macOS/iOS), and Direct3D 12 (Windows). You write your rendering code
once, and wgpu translates it to whichever backend is available at runtime.

**Why not just use OpenGL?** OpenGL was designed in the early 90s and relies on
a lot of global "bind this, then draw" state, making it hard to reason about and
prone to subtle bugs. Modern APIs like Vulkan, Metal, and D3D12 — and by
extension wgpu — are _explicit_: you tell the GPU exactly what resources to use
and in what order, with no hidden state. This is verbose, but it means you
understand exactly what's happening.

**The core mental model:** The CPU and GPU are two separate processors that
don't share memory. To draw something, you:

1. Describe your drawing work as a series of commands (`CommandEncoder`)
2. Hand those commands to the GPU (`Queue::submit`)
3. Tell the swap chain to show the result on screen (`SurfaceTexture::present`)

Everything in `xtal/src/render/gpu.rs` is orchestrating this dance.

---

## 2. The Big Picture: How a Frame Gets to Your Screen

Here is the full lifecycle of a single rendered frame, from "the window is
ready" to "pixels on screen":

```
┌─────────────────────────────────────────────────┐
│                CPU (every frame)                │
│                                                 │
│  1. update sketch (audio, MIDI, time)           │
│  2. set uniform values (resolution, beats, ...) ┼──► GPU uniform buffer
│  3. surface.get_current_texture() ──────────────┼──► SurfaceTexture
│  4. Frame::new(device, queue, output)           │
│  5. graph.execute(...)                          │
│      │                                          │
│      ├─ RenderNode (×N):                        │
│      │   ├─ create_texture_bind_group()         │
│      │   ├─ begin_render_pass()                 │
│      │   ├─ set_pipeline / bind_groups / vbuf   │
│      │   └─ draw()                              │
│      │                                          │
│      ├─ ComputeNode (×N):                       │
│      │   ├─ create_storage_bind_group()         │
│      │   ├─ begin_compute_pass()                │
│      │   └─ dispatch_workgroups()               │
│      │                                          │
│      └─ PresentSource::Texture:                 │
│          └─ blit_texture_to_surface()           │
│                                                 │
│  6. frame.submit() ─────────────────────────────┼──► Queue::submit
│                   ──────────────────────────────┼──► SurfaceTexture::present
└─────────────────────────────────────────────────┘
```

The key insight: **all actual rendering work is recorded into a `CommandEncoder`
first, then submitted to the GPU as a single batch.** The GPU executes that
batch asynchronously while the CPU starts preparing the _next_ frame.

---

## 3. The Render Graph: Declaring What You Want

The `GraphBuilder` in `graph.rs` is a **declarative API** for describing your
rendering pipeline. You say _what_ you want (render this shader to that texture,
then use it as input for another shader), and the system figures out _how_ to
make it happen.

This is the API you use when writing a sketch:

```rust
// graph.rs — GraphBuilder
let mut graph = GraphBuilder::new();

// Step 1: Declare resources
let params   = graph.uniforms();         // the uniform parameter banks
let tex_a    = graph.texture2d();        // an offscreen RGBA texture
let my_img   = graph.image("logo.png"); // a loaded PNG

// Step 2: Declare nodes (operations)
graph.render()
    .shader("my_effect.wgsl")
    .mesh(Mesh::fullscreen_quad())
    .read(params)         // can read uniforms
    .read(my_img)         // can read loaded image
    .to(tex_a);           // writes to tex_a

graph.render()
    .shader("my_display.wgsl")
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(tex_a)          // reads what the first pass wrote
    .to_surface();        // writes directly to the screen

// Step 3: Build the spec
let graph_spec = graph.build();
```

This builder produces a `GraphSpec`, which is just data — no GPU resources yet.
It gets compiled later.

**Why this pattern?** A declarative graph lets the system:

- Validate connections (did you forget to declare a texture before using it?)
- Resize offscreen textures when the window changes without you having to think
  about it
- Execute nodes in the right order
- Know which textures need what usage flags at creation time

---

## 4. Resources: Textures, Uniforms, and Images

Every piece of data that flows through the graph must be **declared as a
resource** with a handle. Handles are just typed integer wrappers — they have no
GPU memory attached to them at build time.

```
                    ResourceHandle
                        │
             ┌──────────┴──────────┐
      UniformHandle           TextureHandle
      (always index 0)        (index 0, 1, 2…)
```

There are three kinds of texture resource:

| Kind                      | What it is                         | When used                              |
| ------------------------- | ---------------------------------- | -------------------------------------- |
| `ResourceKind::Uniforms`  | The uniform parameter buffer       | Reading CPU-side params in shaders     |
| `ResourceKind::Texture2d` | A GPU-side RGBA8 offscreen texture | Intermediate render targets, ping-pong |
| `ResourceKind::Image2d`   | A PNG loaded from disk             | Static images passed to shaders        |

```rust
// graph.rs — resource declaration
pub fn texture2d(&mut self) -> TextureHandle {
    let handle = TextureHandle(self.next_texture_index);
    self.next_texture_index += 1;
    self.resources.push(ResourceDecl {
        handle: ResourceHandle::Texture(handle),
        name: format!("tex{}", handle.0),
        kind: ResourceKind::Texture2d,
    });
    handle
}
```

The returned `TextureHandle` is your receipt. You pass it to `.read()` and
`.to()` calls on nodes to wire up data flow.

---

## 5. Nodes: The Building Blocks of a Graph

The graph has three node types:

### RenderNode

A render node runs a vertex shader and a fragment shader over a mesh. It reads
from some textures and uniforms, and writes color output to a target.

```
  Vertex Buffer                   Fragment Shader Output
  (mesh geometry)  ──► Vertex ──►  Rasterize ──► Fragment ──► Target Texture
                        Shader               │       Shader      (or Surface)
                                             │
                                             └── reads uniforms & input textures
```

Most sketches consist entirely of render nodes, each drawing a **fullscreen
quad** — two triangles that together cover the entire output:

```rust
// mesh.rs
pub fn fullscreen_quad() -> Self {
    Self::Positions2D(vec![
        [-1.0, -1.0],   // bottom-left   ┌──────────┐
        [ 1.0, -1.0],   // bottom-right  │ tri1 /   │
        [-1.0,  1.0],   // top-left      │     / tri2
        [-1.0,  1.0],   // top-left      │    /     │
        [ 1.0, -1.0],   // bottom-right  └──────────┘
        [ 1.0,  1.0],   // top-right
    ])
}
```

The coordinates are in **clip space**: `-1` to `+1` on both axes. The vertex
shader passes these through directly (they already cover the screen), and
converts them to UV coordinates `0..1` for the fragment shader.

### ComputeNode

A compute node runs a **compute shader** that directly writes pixels into a
texture. There's no mesh, no rasterization — the shader just says "for pixel at
coordinate XY, write this color." Covered in detail in
[section 13](#13-compute-shaders-direct-texture-writing).

### Present node

```rust
graph.present(tex_a); // "the final output lives in tex_a"
```

This doesn't execute any shader. It tells the system: when the graph is done,
blit `tex_a` to the screen. If there's no `present()` call and you used
`.to_surface()` directly in a render node, the surface output is already
handled.

---

## 6. Compiling the Graph: From Spec to GPU Objects

`CompiledGraph::compile()` is where the declarative spec turns into real GPU
memory. This happens **once at startup** (or when the sketch is reset).

```rust
// gpu.rs
pub fn compile(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
    graph: GraphSpec,
    uniform_layout: &wgpu::BindGroupLayout,
) -> Result<Self, String>
```

Here's what happens, step by step:

```
GraphSpec
    │
    ├─ find_present_source()         → which texture handle is the final output?
    ├─ collect_texture_resources()   → separate Texture2d and Image2d handles
    ├─ validate_graph_resources()    → are all read/write handles actually declared?
    │
    ├─ For each RenderNodeSpec:
    │    ├─ load & validate WGSL shader from disk (naga parser)
    │    ├─ create_texture_bind_group_layout()   (if reads textures)
    │    ├─ create wgpu::Sampler
    │    ├─ create_render_pipeline()
    │    ├─ upload vertex buffers to GPU (create_mesh_draw)
    │    └─ start ShaderWatch for hot reload
    │
    ├─ For each ComputeNodeSpec:
    │    ├─ load & validate WGSL shader from disk
    │    ├─ create_storage_bind_group_layout()
    │    ├─ create_compute_pipeline()
    │    └─ start ShaderWatch
    │
    └─ For each Image2d resource:
         └─ load_image_texture()   → decode PNG, upload to GPU, store GpuTexture
```

**Important:** Offscreen `Texture2d` resources are **not** created during
compile. They're created lazily in `ensure_offscreen_textures()` on the first
frame, and recreated whenever the window is resized. This is because the texture
size must match the window size, which isn't known at compile time.

Image textures, on the other hand, have a fixed size determined by the file, so
they're uploaded once during compile.

### The Pipeline Creation Deep Dive

`create_render_pipeline()` is the most verbose part of wgpu, but each field has
a purpose:

```rust
// gpu.rs — create_render_pipeline()
device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    layout: Some(&layout),      // which bind groups does this pipeline use?
    vertex: wgpu::VertexState {
        module: &shader,        // the compiled WGSL
        entry_point: Some("vs_main"),
        buffers: &vertex_buffers, // layout of vertex data (2D or 3D positions)
    },
    fragment: Some(wgpu::FragmentState {
        module: &shader,        // same shader module, different entry point
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
            format,             // must match the target texture format
            blend: Some(wgpu::BlendState::ALPHA_BLENDING), // respect alpha
            write_mask: wgpu::ColorWrites::ALL,
        })],
    }),
    primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList, // 3 vertices = 1 triangle
        cull_mode: None,        // don't skip back-facing triangles
        ..
    },
    depth_stencil: None,        // we don't use depth testing
    ..
})
```

The **pipeline layout** describes which bind groups are available and at what
group indices:

```
Pipeline Layout
  ├── group(0): uniform_layout   ← always present (params)
  └── group(1): texture_layout   ← only if the node reads textures
```

The GPU needs this information ahead of time so it can optimize memory access
patterns.

---

## 7. Uniforms: Talking to Your Shaders

Uniforms are small values that are the same for every vertex and every fragment
in a draw call — hence "uniform." In xtal, they carry: resolution, beat timing,
and all the MIDI/OSC parameter controls.

### The UniformBanks Structure

```rust
// uniforms.rs
pub struct UniformBanks {
    data: Vec<[f32; 4]>,        // CPU-side copy
    buffer: wgpu::Buffer,       // GPU-side buffer
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}
```

Each "bank" is a `vec4f` — four 32-bit floats. The data is laid out in memory
like this:

```
Bank index:   0           1           2           3
              ┌───────────┬───────────┬───────────┬───────────┐
              │  a: vec4f │  b: vec4f │  c: vec4f │  d: vec4f │
              │ x  y  z  w│ x  y  z  w│ x  y  z  w│ x  y  z  w│
              └───────────┴───────────┴───────────┴───────────┘
Meaning:       res_w      custom      custom      custom
               res_h
               beats
               dynamic
```

Bank `a` is special: `a.x` and `a.y` are always window width/height, `a.z` is
always beats. `a.w` and all of `b`, `c`, `d` are free for per-sketch parameters.

### Setting Values

```rust
uniforms.set_resolution(1920.0, 1080.0); // → data[0][0..2]
uniforms.set_beats(42.5);                 // → data[0][2]
uniforms.set("bx", 0.75)?;               // → data[1][0]  (bank 'b', component 'x')
uniforms.set("cw", 1.0)?;                // → data[2][3]  (bank 'c', component 'w')
```

The string addressing scheme (`"bx"`, `"cw"`, etc.) is parsed by
`parse_bank_component()`:

- First char: bank letter (`a`=0, `b`=1, `c`=2, …)
- Second char: XYZW component (`x`=0, `y`=1, `z`=2, `w`=3)

### Uploading to GPU

Setting values only updates the CPU-side `data` array. You must call `upload()`
to push it to the GPU:

```rust
// uniforms.rs
pub fn upload(&self, queue: &wgpu::Queue) {
    queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.data));
}
```

`bytemuck::cast_slice` reinterprets the `Vec<[f32; 4]>` as raw bytes. This works
because `[f32; 4]` has a well-defined, packed memory layout. The GPU receives
exactly those bytes.

### In the Shader

```wgsl
// Any WGSL shader
struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let resolution = vec2f(params.a.x, params.a.y);
    let beats      = params.a.z;
    let my_param   = params.b.x;  // set from MIDI/OSC
    // ...
}
```

The `@group(0) @binding(0)` annotation must match the bind group layout used in
the pipeline. Group 0 is always uniforms in xtal.

---

## 8. Bind Groups: The GPU's Way of Passing Data

A **bind group** is a bundle of resources (buffers, textures, samplers) that you
"bind" to a pipeline slot before drawing. Think of it like setting up function
arguments before calling a function.

wgpu uses a two-level system:

- **BindGroupLayout**: describes the _shape_ of the resources (how many, what
  types) — defined at pipeline creation time
- **BindGroup**: the actual resources — can be created and changed at runtime

```
Pipeline Layout                 Bind Groups (runtime)
  group(0) → UniformLayout  →→  BindGroup { buffer: uniform_buffer }
  group(1) → TextureLayout  →→  BindGroup { sampler, texture_view_1, texture_view_2 }
```

### The Uniform Bind Group

Created once in `UniformBanks::new()` and reused every frame. The layout has a
single entry:

```rust
// uniforms.rs
wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::VERTEX
        | wgpu::ShaderStages::FRAGMENT
        | wgpu::ShaderStages::COMPUTE,
    ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: ...,
    },
    count: None,
}
```

Visibility `VERTEX | FRAGMENT | COMPUTE` means shaders at any stage can read
this buffer.

### The Texture Bind Group

Created **every frame** per render node (because texture views could
theoretically change, e.g., after window resize). The layout is generated
dynamically based on how many textures the node reads:

```rust
// gpu.rs — create_texture_bind_group_layout()
// Binding 0: the sampler (how to interpolate between texels)
// Binding 1: first texture
// Binding 2: second texture (if any)
// ... etc.
```

```rust
// gpu.rs — RenderPass::create_texture_bind_group()
let mut entries = vec![
    wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Sampler(sampler),
    },
];

for (index, handle) in sampled_reads.iter().enumerate() {
    let view = // look up the GpuTexture by handle...
    entries.push(wgpu::BindGroupEntry {
        binding: (index + 1) as u32,
        resource: wgpu::BindingResource::TextureView(view),
    });
}
```

In the shader this maps to:

```wgsl
@group(1) @binding(0) var source_sampler: sampler;
@group(1) @binding(1) var source_texture: texture_2d<f32>;
@group(1) @binding(2) var another_texture: texture_2d<f32>;  // if second read
```

### The Compute Storage Bind Group

Compute nodes use a **storage texture** instead of a sampled texture. The
difference:

- **Sampled texture** (`texture_2d<f32>`): read-only, uses a sampler for
  interpolation
- **Storage texture** (`texture_storage_2d<rgba8unorm, write>`): write-only (in
  xtal's case), addressed by exact integer pixel coordinates

```rust
// gpu.rs — create_storage_bind_group_layout()
wgpu::BindingType::StorageTexture {
    access: wgpu::StorageTextureAccess::WriteOnly,
    format: OFFSCREEN_FORMAT, // Rgba8Unorm
    view_dimension: wgpu::TextureViewDimension::D2,
}
```

---

## 9. Shaders: Vertex, Fragment, and Compute

Shaders are programs that run on the GPU. They're written in WGSL (WebGPU
Shading Language) and loaded from `.wgsl` files on disk.

### The Vertex Shader

The vertex shader runs once per vertex in the mesh. For a fullscreen quad that's
6 vertices. Its job: transform the input position into clip space, and pass any
data along to the fragment shader.

```wgsl
// From templates/basic.wgsl
struct VertexInput {
    @location(0) position: vec2f,   // from the vertex buffer
}

struct VsOut {
    @builtin(position) position: vec4f,  // clip-space position (required)
    @location(0) uv: vec2f,              // passed to fragment shader
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;  // already in clip space (-1..1)
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);      // z=0, w=1 (no perspective)
    out.uv = p * 0.5 + vec2f(0.5, 0.5);    // remap -1..1 → 0..1
    return out;
}
```

The `@location(0)` on `position` in `VertexInput` must match what the vertex
buffer layout says is at attribute index 0. In xtal, that's always a `Float32x2`
(for 2D meshes) or `Float32x3` (for 3D meshes).

### The Fragment Shader

The fragment shader runs once per pixel covered by a rasterized triangle. It
receives interpolated values from the vertex shader and outputs a color.

```wgsl
// From templates/basic.wgsl
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let p = (in.uv * resolution - 0.5 * resolution) / resolution.y;
    // p is now centered and aspect-ratio corrected

    let beats = params.a.z;
    let radius = max(0.05, params.a.w);

    let pulse = 0.55 + 0.45 * sin(beats * 4.0);
    let d = length(p);
    let ring = smoothstep(radius, radius - 0.015, d);

    let bg = vec3f(0.0, 0.0, 0.8);
    let fg = vec3f(0.15, 0.85, 0.0) * pulse;
    var color = mix(bg, fg, ring);

    return vec4f(color, 1.0);  // RGBA, alpha=1
}
```

**UV vs. `p` (centered, aspect-corrected):**

- `uv` goes from `(0,0)` top-left to `(1,1)` bottom-right
- `p` is the centered, aspect-corrected version: `(0,0)` is screen center, and
  the vertical range is always `-0.5..0.5`, while horizontal range depends on
  aspect ratio. This is useful for effects that should look the same regardless
  of window size.

### Reading Input Textures

When a render node reads a previous pass's output texture, the fragment shader
can sample it:

```wgsl
// From templates/feedback.wgsl
@group(1) @binding(0) var source_sampler: sampler;
@group(1) @binding(1) var source_texture: texture_2d<f32>;

@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let previous_frame = textureSample(source_texture, source_sampler, uv);
    // ...
}
```

`textureSample` interpolates between texels using the sampler. In xtal the
sampler uses `FilterMode::Linear`, so sampling between two pixels gives a smooth
blend.

### Shader Validation

Before a shader is handed to the GPU driver, it's parsed by `naga` (the shader
translation library that underlies wgpu) to catch errors early:

```rust
// gpu.rs
fn validate_shader(source: &str) -> Result<(), String> {
    let module = wgsl::parse_str(source).map_err(|err| err.to_string())?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.validate(&module).map_err(|err| err.to_string()).map(|_| ())
}
```

This runs at startup for every shader and also during hot reload. If your shader
has a type error or uses a wrong bind group layout, you get a clear error
message rather than a GPU crash.

---

## 10. The Frame Loop: Executing the Graph Every Frame

Every frame, `CompiledGraph::execute()` is called. Here's what happens in
detail:

```rust
// gpu.rs
pub fn execute(
    &mut self,
    device: &wgpu::Device,
    frame: &mut Frame,
    uniforms: &UniformBanks,
    surface_size: [u32; 2],
) -> Result<(), String>
```

### Step 1: Ensure Offscreen Textures Exist

```rust
self.ensure_offscreen_textures(device, surface_size);
```

On the first frame, or after a window resize, offscreen textures are
(re)created. Their size must match `surface_size` so UV coordinates line up with
the screen.

```rust
// gpu.rs — ensure_offscreen_textures()
let texture = device.create_texture(&wgpu::TextureDescriptor {
    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    format: OFFSCREEN_FORMAT,           // Rgba8Unorm
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT  // can be written by render passes
        | wgpu::TextureUsages::TEXTURE_BINDING     // can be read by fragment shaders
        | wgpu::TextureUsages::STORAGE_BINDING     // can be written by compute shaders
        | wgpu::TextureUsages::COPY_SRC,           // can be copied out (for recording)
    ..
});
```

Note the four usage flags. On modern GPUs, textures need to declare ahead of
time how they'll be used. Getting this wrong causes validation errors.

### Step 2: Execute Each Node

```rust
for node in &mut self.nodes {
    match node {
        CompiledNode::Render(node) => { /* render pass */ }
        CompiledNode::Compute(node) => { /* compute pass */ }
    }
}
```

For a render node:

```rust
// 1. Hot-reload the shader if changed on disk
node.pass.update_if_changed(device, &node.sampled_reads, uniforms.bind_group_layout());

// 2. Build the per-frame texture bind group
let texture_bind_group = node.pass.create_texture_bind_group(...)?;

// 3. Determine the target view
let target_view = match node.target {
    RenderTarget::Surface => frame.surface_view.clone(),
    RenderTarget::Texture(handle) => self.offscreen_textures[&handle].view.clone(),
};

// 4. Begin the render pass
let mut render_pass = frame.encoder().begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &target_view,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // clear to black first
            store: wgpu::StoreOp::Store,                   // keep the result
        },
    })],
    ..
});

// 5. Set pipeline and bind groups
render_pass.set_pipeline(&node.pass.render_pipeline);
render_pass.set_bind_group(0, uniforms.bind_group(), &[]);  // group 0: uniforms
render_pass.set_bind_group(1, &texture_bind_group, &[]);    // group 1: textures

// 6. Draw each mesh
for mesh in &node.pass.meshes {
    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    render_pass.draw(0..mesh.vertex_count, 0..1);
}
```

The render pass is **automatically ended** when `render_pass` is dropped at the
end of the block (Rust's RAII). This records an "end render pass" command into
the encoder.

### The Frame Object

`Frame` is a thin wrapper that owns the `CommandEncoder` and `SurfaceTexture`
for one frame:

```rust
// frame.rs
pub struct Frame {
    pub surface_view: wgpu::TextureView,  // view into the swap chain texture
    encoder: Option<wgpu::CommandEncoder>, // records all commands
    output: Option<wgpu::SurfaceTexture>,  // the swap chain texture
    queue: Arc<wgpu::Queue>,
}
```

When `frame.submit()` is called at the end:

```rust
pub fn submit(mut self) -> wgpu::SubmissionIndex {
    let encoder = self.encoder.take().unwrap();
    let submission_index = self.queue.submit(Some(encoder.finish()));
    if let Some(output) = self.output.take() {
        output.present();  // tell the OS to show this texture on screen
    }
    submission_index
}
```

`encoder.finish()` seals the command buffer. `queue.submit()` hands it to the
GPU. `output.present()` hands the swap chain texture to the display compositor.
After this, the GPU works asynchronously while the CPU starts on the next frame.

---

## 11. Offscreen Textures and the Render Target System

When a render pass writes to an offscreen texture instead of the surface, that
texture stays on the GPU and can be read by subsequent passes in the same frame.
This is the core mechanism for multi-pass rendering.

```
Frame N:
  ┌──────────┐  writes   ┌────────────┐  reads    ┌──────────────┐  writes  ┌─────────┐
  │  Pass 1  │ ────────► │  Tex A     │ ────────► │   Pass 2     │ ────────►│ Surface │
  │ effect.  │           │ (offscreen)│           │ post.wgsl    │          │ (screen)│
  │ wgsl     │           └────────────┘           └──────────────┘          └─────────┘
```

### Texture Formats

xtal uses two texture formats, and their distinction matters:

| Format           | Where used                         | Meaning                                              |
| ---------------- | ---------------------------------- | ---------------------------------------------------- |
| `Rgba8Unorm`     | Offscreen intermediate textures    | 4 bytes/pixel, values 0.0–1.0, linear color space    |
| `Rgba8UnormSrgb` | Surface (screen) and loaded images | Same bytes, but GPU auto-converts to/from sRGB gamma |

The surface uses sRGB because monitors expect gamma-encoded color. But doing
math in a gamma-encoded space produces incorrect blending. By keeping
intermediate textures in linear `Rgba8Unorm`, all the shader math is done in
linear light. The final blit to the sRGB surface applies gamma correction
automatically.

This is why there's a `blit_texture_to_surface()` step — you can't just copy
bytes, you need the format conversion.

### Lazily Created, Eagerly Replaced

```rust
// gpu.rs — ensure_offscreen_textures()
let needs_new = self.offscreen_textures
    .get(handle)
    .is_none_or(|texture| texture.size != [width, height]);

if !needs_new { continue; }

// create new texture at current size...
self.offscreen_textures.insert(*handle, GpuTexture { texture, view, size, format });
```

When the window is resized, `size != [width, height]` is true for all existing
textures. They're all replaced. The old `wgpu::Texture` objects are dropped,
which releases the GPU memory.

---

## 12. Ping-Pong and Feedback Effects

A **feedback effect** reads the _previous_ frame's output and blends it with new
content. This creates trails, glows, and temporal blur. The technique requires
two textures so you can read from one while writing to the other.

```rust
// graph.rs — how to set up feedback
let (fb_read, fb_write) = graph.feedback();
// This is just:
// fb_read  = graph.texture2d()   ← frame N-1's result
// fb_write = graph.texture2d()   ← frame N's result
```

The graph for a feedback sketch looks like this:

```rust
let mut graph = GraphBuilder::new();
let params = graph.uniforms();
let (fb_read, fb_write) = graph.feedback();

graph.render()
    .shader("feedback.wgsl")
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(fb_read)    // ← reads last frame's output
    .to(fb_write);    // ← writes this frame's new output

graph.present(fb_write);
```

**The ping-pong swap:** After each frame, `fb_write` contains the new image. On
the next frame, we want `fb_read` to contain it. But the handles are fixed —
`fb_read` always points to the same texture.

How does this work? The **sketch** is responsible for swapping the handles
between frames in its `update()` method. The two textures alternate roles each
frame:

```
Frame 1:  fb_read=tex0 (empty), fb_write=tex1  →  tex1 gets frame 1 content
Frame 2:  fb_read=tex1 (frame1), fb_write=tex0 →  tex0 gets frame 2 content (blended with frame1)
Frame 3:  fb_read=tex0 (frame2), fb_write=tex1 →  tex1 gets frame 3 content (blended with frame2)
```

In the shader, the blend looks like this (from `templates/feedback.wgsl`):

```wgsl
@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let feedback_mix = params.a.w;  // 0.0 = no feedback, 0.99 = heavy trails
    let zoom = params.b.x;

    // Zoom slightly to prevent hard edges at the border
    let centered_uv = (uv - 0.5) * zoom + 0.5;

    // Sample the previous frame
    let fb = textureSample(source_texture, source_sampler, centered_uv).rgb;

    // Draw new content (a glowing ring)
    let ring = /* ... */;
    let glow = vec3f(...) * ring;

    // Mix: most of what we see is last frame, plus the new ring
    let color = fb * feedback_mix + glow;
    return vec4f(color, 1.0);
}
```

When `feedback_mix` is `0.98`, 98% of each pixel comes from the previous frame.
The ring burns itself into the image and fades over many frames, creating a
luminous trail.

---

## 13. Compute Shaders: Direct Texture Writing

Compute shaders bypass the render pipeline entirely. Instead of drawing
triangles and having the GPU figure out which pixels they cover (rasterization),
compute shaders let you say: "run this function once for every pixel."

```wgsl
// templates/compute.wgsl
@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var field: texture_storage_2d<rgba8unorm, write>;  // write-only storage texture

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let dim = textureDimensions(field);

    // Guard: don't write outside the texture
    if (gid.x >= dim.x || gid.y >= dim.y) { return; }

    let uv = vec2f(gid.xy) / vec2f(dim);  // normalize to 0..1
    let p  = uv * 2.0 - vec2f(1.0);       // remap to -1..1

    let t    = params.a.z;   // beats
    let freq = params.a.w;

    // Some math to generate a color at this pixel
    let wave_a = sin((p.x + p.y * 0.2) * 12.0 * freq + t * 1.5);
    let color  = vec3f(0.5 + 0.5 * wave_a, ...);

    textureStore(field, vec2i(gid.xy), vec4f(color, 1.0));
}
```

### Workgroups: How Compute Parallelism Works

The GPU executes compute shaders in **workgroups** — small, tightly-coupled
groups of shader invocations that run simultaneously on the same compute unit.
In xtal, the workgroup size is `(8, 8, 1)` meaning each workgroup processes an
8×8 block of pixels (64 pixels at once).

To cover the entire texture, the CPU dispatches enough workgroups:

```rust
// gpu.rs
let workgroup_x = width.div_ceil(8);   // e.g., 1920 / 8 = 240 workgroups wide
let workgroup_y = height.div_ceil(8);  // e.g., 1080 / 8 = 135 workgroups tall

compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
```

```
Texture (simplified 24×16):

  WG(0,0)  WG(1,0)  WG(2,0)  ← each workgroup = 8×8 pixels
  ┌──────┬──────┬──────┐
  │      │      │      │
  ├──────┼──────┼──────┤
  │      │      │      │
  └──────┴──────┴──────┘
  WG(0,1)  WG(1,1)  WG(2,1)
```

Each shader invocation gets its own `gid` (global invocation ID), which is its
pixel coordinate within the full texture. The guard
`if (gid.x >= dim.x || gid.y >= dim.y) { return; }` handles the edge case where
the texture width isn't exactly divisible by 8.

### Using a Compute Node in the Graph

```rust
let mut graph = GraphBuilder::new();
let params = graph.uniforms();
let field  = graph.texture2d();  // the texture the compute shader writes to

graph.compute()
    .shader("my_compute.wgsl")
    .read_write(field)            // this texture is both the target and output
    .dispatch();

// Then display it
graph.present(field);
```

The `read_write` name is a bit misleading — currently xtal only binds compute
textures as `write`-only storage textures. The shader writes to it, and it's
then read by a subsequent render pass or blit.

---

## 14. The Present Blit: Getting Offscreen Work onto the Screen

When you call `graph.present(some_texture)`, the compiled graph remembers that
`some_texture` is the final output. After all nodes execute,
`blit_texture_to_surface()` runs a mini render pipeline to copy that texture to
the swap chain surface.

```rust
// gpu.rs — called at the end of execute()
if let PresentSource::Texture(source) = self.present_source {
    blit_texture_to_surface(device, frame, &source_view, self.surface_format);
}
```

`blit_texture_to_surface()` creates a **temporary** render pipeline every frame
using a hard-coded WGSL shader embedded in the Rust source:

```rust
// gpu.rs — PRESENT_BLIT_WGSL (embedded in the binary)
const PRESENT_BLIT_WGSL: &str = r#"
@group(0) @binding(0) var tex_sampler: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    // Generates a fullscreen quad WITHOUT a vertex buffer
    // by computing positions from the vertex index
    var positions = array<vec2f, 4>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0),
        vec2f(-1.0,  1.0), vec2f(1.0,  1.0),
    );
    let p = positions[vertex_index];
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, in.uv);  // simple copy
}
"#;
```

A few things worth noting:

1. **No vertex buffer:** The vertex shader uses `@builtin(vertex_index)` to
   compute positions procedurally. `draw(0..4, 0..1)` is called — four
   "vertices" with no data, just indices 0, 1, 2, 3.

2. **Triangle strip topology:** The present pipeline uses `TriangleStrip`
   instead of `TriangleList`. With a strip, 4 vertices form 2 triangles that
   share an edge, perfectly covering the screen. The content render pipeline
   uses `TriangleList` (6 explicit vertices for 2 triangles).

3. **Format conversion:** The source texture is `Rgba8Unorm` (linear). The
   surface target is `Rgba8UnormSrgb`. The GPU applies gamma correction
   automatically during this blit — linear values are encoded to sRGB gamma for
   correct display brightness.

4. **`REPLACE` blend mode:** Unlike content passes which use `ALPHA_BLENDING`,
   the blit uses `BlendState::REPLACE` — it overwrites the surface completely.

---

## 15. Shader Hot Reload

One of xtal's nicest features for creative coding is that shaders reload
automatically when you save a `.wgsl` file. You can tweak a parameter or
algorithm and see the result instantly without restarting.

### How It Works

Each `RenderPass` and `ComputePass` holds an optional `ShaderWatch`:

```rust
// gpu.rs
struct RenderPass {
    // ...
    watcher: Option<ShaderWatch>,
}
```

`ShaderWatch` (in `shader_watch.rs`) uses the `notify` crate to watch the
shader's parent directory for filesystem events:

```rust
// shader_watch.rs
pub struct ShaderWatch {
    changed: Arc<AtomicBool>,
    _watcher: RecommendedWatcher,  // underscore prefix = held alive, not used directly
}
```

The `_watcher` field is crucial — it's held alive by the struct, so the OS-level
file watch stays active. When a filesystem event fires, the callback sets
`changed` to `true`:

```rust
changed_flag.store(true, Ordering::SeqCst);
```

An `AtomicBool` is used so the filesystem-event callback thread (the `notify`
watcher runs on a background thread) can safely write to a flag that the main
render thread reads.

### Content Hashing

To avoid spurious reloads (some editors write files in multiple steps), the
watcher computes a hash of the file content before marking it as changed:

```rust
// shader_watch.rs
fn file_content_hash(path: &Path) -> Result<u64, std::io::Error> {
    let bytes = fs::read(path)?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}
```

If the new hash matches the previously-loaded hash, the reload is skipped — the
file changed on disk but the content is the same (common with editor autosave
behavior).

### Applying the Reload

At the start of each frame, before executing any nodes, `update_if_changed()` is
called:

```rust
// gpu.rs — RenderPass::update_if_changed()
fn update_if_changed(&mut self, device: &wgpu::Device, ...) {
    if !self.watcher.as_ref().is_some_and(ShaderWatch::take_changed) {
        return;  // nothing changed, fast path
    }

    // Re-read, re-validate, re-compile
    let source = fs::read_to_string(&self.shader_path)?;
    validate_shader(&source)?;

    self.render_pipeline = create_render_pipeline(
        device, self.target_format, self.mesh_kind,
        uniform_layout, self.texture_bind_group_layout.as_ref(),
        &source, "xtal-hot-reloaded",
    );
}
```

`take_changed()` atomically reads and resets the flag:

```rust
pub fn take_changed(&self) -> bool {
    self.changed.swap(false, Ordering::SeqCst)
}
```

If the shader has a parse error, validation fails and the old pipeline continues
to be used — the error is logged but the app keeps running. This is essential
for creative coding where partial edits are common.

---

## 16. Row Padding and Memory Alignment

When copying texture data from the GPU back to the CPU (for recording video or
taking screenshots), there's a subtle complication: the GPU requires that each
row of texture data start on a 256-byte boundary.

```rust
// gpu.rs
pub fn compute_row_padding(unpadded_bytes_per_row: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
    let rem = unpadded_bytes_per_row % align;
    if rem == 0 { 0 } else { align - rem }
}
```

For a 1920×1080 texture at 4 bytes/pixel:

- Unpadded bytes per row: `1920 × 4 = 7680`
- `7680 % 256 = 0` → no padding needed (7680 is exactly divisible by 256)

For a 1000-pixel-wide texture:

- Unpadded: `1000 × 4 = 4000`
- `4000 % 256 = 160` → padding needed: `256 - 160 = 96` extra bytes per row
- Padded: `4096` bytes per row (even though only 4000 contain pixel data)

This padding only matters when copying _to a buffer_ for CPU readback. When
rendering to a texture, wgpu handles alignment internally.

---

## 17. Putting It All Together: Walkthrough of a Multi-Pass Sketch

Let's trace through what happens when you run a sketch that uses feedback.
Here's a simplified version of the feedback sketch:

### Sketch Setup (once at startup)

```rust
// In the sketch's setup() or init() function:
let mut graph = GraphBuilder::new();
let params   = graph.uniforms();
let (fb_read, fb_write) = graph.feedback();
// fb_read  → TextureHandle(0), tex0
// fb_write → TextureHandle(1), tex1

graph.render()
    .shader("feedback.wgsl")
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(fb_read)
    .to(fb_write);

graph.present(fb_write);

let spec = graph.build();
let compiled_graph = CompiledGraph::compile(device, queue, surface_format, spec, &uniform_layout)?;
```

What `compile()` does:

- Validates the resource graph (fb_read and fb_write are declared, present
  source is fb_write ✓)
- Reads `feedback.wgsl` from disk, parses with naga, validates ✓
- Creates a `wgpu::Sampler` (for sampling fb_read in the shader)
- Creates a `BindGroupLayout` for the texture group: `[sampler, texture_2d]`
- Creates the full `wgpu::RenderPipeline` (vertex + fragment stages, blend mode,
  format)
- Uploads the 6-vertex fullscreen quad to a GPU vertex buffer
- Starts a `ShaderWatch` on `feedback.wgsl`'s directory
- Records `fb_write` (TextureHandle(1)) as the present source
- Note: **does NOT create** the actual offscreen textures yet

### Frame 1

```
CPU:
  1. uniforms.set_resolution(1920, 1080)
     uniforms.set_beats(0.0)
     uniforms.set("aw", 0.97)   // feedback_mix = 97%
     uniforms.upload(queue)     // → writes 64 bytes to GPU buffer

  2. output = surface.get_current_texture()
     frame = Frame::new(device, queue, output)
     // frame.surface_view points to the swap chain texture

  3. graph.execute(device, &mut frame, &uniforms, [1920, 1080])

     3a. ensure_offscreen_textures([1920, 1080])
         → creates tex0 (1920×1080, Rgba8Unorm) on GPU  ← fb_read
         → creates tex1 (1920×1080, Rgba8Unorm) on GPU  ← fb_write

     3b. Execute RenderNode "render_0" (feedback.wgsl → fb_write):
         - check hot reload → no change
         - create texture bind group: { sampler, tex0.view }
                                           ↑ this is fb_read = tex0
         - target_view = tex1.view  (fb_write)
         - encoder.begin_render_pass(target=tex1, clear=BLACK)
         - set_pipeline(feedback_pipeline)
         - set_bind_group(0, uniform_bind_group)   // params
         - set_bind_group(1, texture_bind_group)   // sampler + tex0
         - set_vertex_buffer(0, quad_buffer)
         - draw(0..6, 0..1)
         - [render pass ends when dropped]
         // GPU will: for each of the 6 vertices, run vs_main
         //           for each covered pixel, run fs_main
         // fs_main reads tex0 (all black, frame 0), blends with the ring effect
         // output goes to tex1

     3c. PresentSource::Texture(fb_write=tex1):
         blit_texture_to_surface(device, frame, tex1.view, sRGB_format)
         → creates temporary pipeline + bind group
         → encoder.begin_render_pass(target=frame.surface_view, clear=BLACK)
         → draw(0..4, 0..1)  [fullscreen triangle strip, no vertex buffer]
         → copies tex1 → swap chain, with Unorm→sRGB gamma conversion

  4. frame.submit()
     → encoder.finish() seals all recorded commands
     → queue.submit(command_buffer) hands to GPU
     → output.present() tells OS to display this frame
```

### Frame 2

The sketch swaps `fb_read` ↔ `fb_write` (so what was written becomes the next
read source). Now:

- `fb_read` → tex1 (contains frame 1's result)
- `fb_write` → tex0 (will be written this frame)

The process repeats. When `fs_main` samples `source_texture`, it now reads tex1
(which has the ring from frame 1). It blends 97% of that with the new ring
position, creating a fading trail.

### Data Flow Diagram (steady state)

```
                     ┌─────────── GPU ───────────┐
                     │                            │
  CPU:               │  tex0 ◄─── written         │
  uniforms ─────────►│  (Rgba8Unorm)             │
  (beats, params)    │                            │
                     │  tex1 ─────── sampled ────►│
                     │  (prev frame)              │
                     │                            │
                     │  ┌─────────────────────┐   │
                     │  │ feedback.wgsl       │   │
                     │  │ fs_main():          │   │
                     │  │  fb = sample(tex1)  │   │
                     │  │  ring = draw_ring() │   │
                     │  │  out = fb*0.97+ring │   │
                     │  └────────┬────────────┘   │
                     │           │ writes          │
                     │           ▼                 │
                     │        tex0                 │
                     │           │ blit            │
                     │           ▼                 │
                     │       Surface               │
                     │   (swap chain, sRGB)        │
                     └─────────────────────────────┘

Next frame: tex0 and tex1 swap roles.
```

---

## Summary: The Key Concepts

| Concept              | Where in code     | What it does                                                          |
| -------------------- | ----------------- | --------------------------------------------------------------------- |
| `GraphBuilder`       | `graph.rs`        | Declarative builder for describing your render pipeline               |
| `CompiledGraph`      | `gpu.rs`          | Compiled GPU state: pipelines, buffers, textures                      |
| `UniformBanks`       | `uniforms.rs`     | Passes CPU data (resolution, beats, MIDI) to all shaders              |
| `Frame`              | `frame.rs`        | Owns the `CommandEncoder` and `SurfaceTexture` for one frame          |
| `RenderNode`         | `gpu.rs`          | A draw call: vertex/fragment shaders + mesh → texture                 |
| `ComputeNode`        | `gpu.rs`          | A compute dispatch: compute shader → texture                          |
| `OFFSCREEN_FORMAT`   | `gpu.rs`          | `Rgba8Unorm` — linear color for math-correct intermediate textures    |
| Bind group           | `gpu.rs`          | Bundle of resources handed to a pipeline slot                         |
| `ShaderWatch`        | `shader_watch.rs` | Background file watcher that flags shaders for hot reload             |
| Feedback / ping-pong | `graph.rs`        | Two textures that alternate as read/write each frame                  |
| Present blit         | `gpu.rs`          | Copies final offscreen texture to the swap chain with sRGB conversion |

The overall design philosophy is: **declare resources and dataflow at startup,
execute the graph cheaply every frame.** All the expensive work (pipeline
compilation, texture allocation, shader loading) happens once. The per-frame
loop just records commands and submits them.
