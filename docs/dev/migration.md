# xtal2 POC: Nannou-free shader framework

## Context

Xtal has evolved into a live shader coding tool, but is built on Nannou 0.19.0
which is unmaintained. The actual Nannou surface area is small (window/device/
queue/frame), and 90% of sketches follow an identical pattern where the Rust file
is pure boilerplate. The goal is to replace Nannou with raw winit+wgpu and
radically reduce per-sketch ceremony so that a sketch is primarily defined by its
**.yaml** and **.wgsl** files, with a minimal Rust config file.

## POC Scope

A single fullscreen shader running end-to-end:
- winit window + wgpu surface (no Nannou)
- Hot-reloading `.wgsl` shaders (port the notify watcher)
- Uniform banks system working (hardcoded params for now, no ControlHub)
- The new sketch definition pattern proven out

No ControlHub, no UI, no MIDI/OSC, no recording, no sketch switching.

## Target Sketch Definition (the north star)

For the 90% fullscreen-shader case, the entire Rust file should be:

```rust
use xtal2::prelude::*;

pub const CONFIG: SketchConfig = SketchConfig {
    name: "my_shader",
    display_name: "My Shader",
    bpm: 134.0,
    fps: 60.0,
    w: 700,
    h: 700,
    banks: 4,
};
```

That's it. The framework:
- Finds `my_shader.yaml` and `my_shader.wgsl` co-located with the `.rs` file
- Generates the `ShaderParams` struct with 4 banks
- Handles init, update (populating params from hub), view (render to frame)
- Wires up beats into `a3` automatically

For the POC, since we don't have ControlHub yet, the sketch will look slightly
different (we'll hardcode a time/beats value). But the structure should be
forward-compatible with the above vision.

### POC Sketch Definition

```rust
use xtal2::prelude::*;

pub const CONFIG: SketchConfig = SketchConfig {
    name: "demo",
    display_name: "Demo",
    fps: 60.0,
    w: 700,
    h: 700,
    banks: 4,
};
```

The framework auto-provides resolution (a1, a2) and elapsed beats (a3). Bank a4
onward are zeros until ControlHub is integrated later.

### Non-trivial sketches (procedural, feedback, custom logic)

These still need a struct + trait impl. The full `Sketch` trait remains available:

```rust
pub trait Sketch {
    fn update(&mut self, ctx: &Context) {}
    fn view(&mut self, frame: &mut Frame, ctx: &Context);
}
```

This is a future concern, not part of the POC.

## Files to Create

```
xtal-project/xtal2/
  Cargo.toml
  src/
    lib.rs              # re-exports, prelude
    prelude.rs          # convenience imports
    app.rs              # winit event loop, run()
    context.rs          # Context: device, queue, window size, timing
    frame.rs            # Frame: wraps surface texture + command encoder
    gpu.rs              # GpuState: pipeline, buffers, render (ported from xtal)
    shader_watch.rs     # notify file watcher for hot-reload
    sketch.rs           # SketchConfig, Sketch trait
    uniforms.rs         # runtime uniform bank generation (no proc macro needed)
  examples/
    demo/
      main.rs           # entry point: creates config, calls xtal2::run()
      demo.wgsl         # example fullscreen shader
```

## Implementation Steps

### Step 1: Cargo.toml + workspace integration

Add `xtal2` to the workspace members in
`/Users/lokua/code/xtal-project/Cargo.toml`.

**xtal2/Cargo.toml dependencies:**
- `wgpu` (same version as nannou uses, or latest stable)
- `winit` (latest 0.29.x or 0.30.x — check what wgpu expects)
- `bytemuck` (Pod/Zeroable for uniform buffers)
- `naga` (shader validation before hot-reload)
- `notify` (file watcher for shader hot-reload)
- `log` + `env_logger` (logging)
- `pollster` (block on async wgpu init)

### Step 2: sketch.rs — SketchConfig

```rust
pub struct SketchConfig {
    pub name: &'static str,
    pub display_name: &'static str,
    pub fps: f32,
    pub w: u32,
    pub h: u32,
    pub banks: usize,
}
```

No `bpm` or `play_mode` yet (those depend on timing/ControlHub).

Trait definition:
```rust
pub trait Sketch {
    fn update(&mut self, ctx: &Context) {}
    fn view(&mut self, frame: &mut Frame, ctx: &Context);
}
```

### Step 3: context.rs — Context

Holds GPU handles and window state. Replaces `App` + `Context` from xtal.

```rust
pub struct Context {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    window_size: [u32; 2],
    scale_factor: f64,
    frame_count: u64,
    start_time: Instant,
}
```

Key methods:
- `resolution() -> [f32; 2]`
- `resolution_u32() -> [u32; 2]`
- `elapsed_seconds() -> f32`
- `frame_count() -> u64`

### Step 4: frame.rs — Frame

Wraps the wgpu surface texture for a single frame. Created by the run loop,
passed to `view()`.

```rust
pub struct Frame {
    pub surface_view: wgpu::TextureView,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    encoder: Option<wgpu::CommandEncoder>,
    format: wgpu::TextureFormat,
}
```

Key methods:
- `encoder(&mut self) -> &mut wgpu::CommandEncoder`
- `submit(self)` — finishes encoder, submits to queue

### Step 5: uniforms.rs — Runtime uniform banks (no proc macro)

Instead of a proc macro generating a struct, generate the uniform buffer at
runtime from the `banks` count in SketchConfig.

```rust
pub struct UniformBanks {
    data: Vec<[f32; 4]>,
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}
```

Key methods:
- `new(device, banks: usize) -> Self`
- `set_resolution(&mut self, w: f32, h: f32)` — sets a[0], a[1]
- `set_beats(&mut self, beats: f32)` — sets a[2]
- `set(&mut self, bank: &str, value: f32)` — e.g. "a4", "b1"
- `upload(&self, queue)` — write_buffer to GPU

This is simpler than the proc macro approach and works for the convention-based
sketches. The data layout is identical: N contiguous `[f32; 4]` arrays, matching
the WGSL `Params { a: vec4f, b: vec4f, ... }` struct.

### Step 6: gpu.rs — GpuState (ported, simplified)

Port from `xtal/src/framework/gpu.rs`, replacing all Nannou types:
- `app.main_window().device()` → `ctx.device` / `Arc<wgpu::Device>`
- `app.main_window().queue()` → `ctx.queue` / `Arc<wgpu::Queue>`
- `Frame::TEXTURE_FORMAT` → the surface format from configuration
- `frame.command_encoder()` → `frame.encoder()`
- `frame.texture_view()` → `frame.surface_view`
- Nannou's `wgpu::TextureBuilder` → raw wgpu texture creation
- Nannou's `wgpu::RenderPassBuilder` → raw wgpu render pass descriptors
- Nannou's `wgpu::BindGroupBuilder` → raw wgpu bind group creation

For the POC, only implement fullscreen mode. Keep the structure extensible for
procedural/texture modes later.

Drop `bevy_reflect` — it was only used for vertex attribute inference on custom
vertex types. Fullscreen uses a hardcoded quad layout, procedural has no
vertices. Can be re-added later if custom vertex types are needed.

### Step 7: shader_watch.rs — Hot-reload watcher

Direct port from the `start_shader_watcher` + `validate_shader` logic in
`xtal/src/framework/gpu.rs`. No Nannou dependencies in this code. Uses `notify`
crate and `naga` for validation.

Note: the watcher accepts `Create` and `Modify` events (not just
`Modify(Data(Content))`) to handle atomic-write editors that rename temp files.

### Step 8: app.rs — The run loop

The core event loop using winit + wgpu:

```
1. Create winit EventLoop + Window from SketchConfig (size, title)
2. Create wgpu Instance → Surface → Adapter → Device + Queue
3. Configure surface (format, present mode, size)
4. Create Context (device, queue, window size, scale factor)
5. Create GpuState for fullscreen shader (from SketchConfig.name → .wgsl path)
6. Create UniformBanks (from SketchConfig.banks)
7. Enter event loop:
   WindowEvent::Resized → reconfigure surface, update context
   WindowEvent::RedrawRequested →
     a. Update uniforms (resolution, time/beats)
     b. Get surface texture → create Frame
     c. GpuState renders fullscreen quad with uniforms
     d. Frame submits encoder
     e. Surface texture present
     f. Request redraw
   WindowEvent::CloseRequested → exit
```

Public API:
```rust
pub fn run(config: &'static SketchConfig, shader_dir: PathBuf)
```

The `shader_dir` is derived from `file!()` at the call site. We'll provide a
convenience macro:

```rust
#[macro_export]
macro_rules! run {
    ($config:expr) => {
        xtal2::app::run(
            &$config,
            xtal2::shader_dir!(file!()),
        )
    };
}
```

Where `shader_dir!` extracts the parent directory from `file!()`.

### Step 9: lib.rs + prelude.rs

```rust
// lib.rs
pub mod app;
pub mod context;
pub mod frame;
pub mod gpu;
pub mod prelude;
pub mod shader_watch;
pub mod sketch;
pub mod uniforms;
```

```rust
// prelude.rs
pub use crate::sketch::*;
pub use crate::context::Context;
pub use crate::frame::Frame;
```

### Step 10: Example sketch

**examples/demo/main.rs:**
```rust
use xtal2::prelude::*;

const CONFIG: SketchConfig = SketchConfig {
    name: "demo",
    display_name: "Demo",
    fps: 60.0,
    w: 700,
    h: 700,
    banks: 4,
};

fn main() {
    xtal2::run!(&CONFIG);
}
```

**examples/demo/demo.wgsl:**
A simple shader using the uniform banks pattern — circle with `params.a.w`
controlling radius, `params.a.z` as time/beats.

## Key Porting Decisions

### Nannou wgpu re-exports vs raw wgpu

Nannou re-exports wgpu with its own builder extensions (TextureBuilder,
RenderPassBuilder, BindGroupBuilder). In xtal2, use raw wgpu APIs directly.
The translation is straightforward:

| Nannou pattern | Raw wgpu equivalent |
|---|---|
| `TextureBuilder::new().size(s).format(f).build(device)` | `device.create_texture(&TextureDescriptor { ... })` |
| `RenderPassBuilder::new().color_attachment(view, ...).begin(&mut enc)` | `encoder.begin_render_pass(&RenderPassDescriptor { ... })` |
| `frame.command_encoder()` | `device.create_command_encoder(...)` (owned by Frame) |
| `texture.view().build()` | `texture.create_view(&TextureViewDescriptor::default())` |

### No MSAA for POC

Nannou defaults to MSAA. For the POC, `sample_count: 1`. Can add MSAA later
as a SketchConfig option.

### Surface format

Use `surface.get_capabilities(adapter).formats[0]` or prefer `Bgra8UnormSrgb`
if available (consistent with Nannou). Store in Context.

### No vertex type parameter

`GpuState<V>` had a generic vertex type. For the POC (fullscreen only), this
is always the hardcoded quad. No generic needed. If procedural mode is added
later, it can use `GpuState` without a vertex buffer (just draw N vertices).

## Verification

1. `cargo check -p xtal2` compiles
2. `cargo run -p xtal2 --example demo` opens a window showing the shader
3. Edit `demo.wgsl` while running → shader hot-reloads without restart
4. Resize window → shader adapts (resolution uniform updates)
5. Close window → clean exit

## Files Modified (existing)

- `/Users/lokua/code/xtal-project/Cargo.toml` — add `"xtal2"` to workspace
  members

## Files Referenced (for porting)

- `xtal/src/framework/gpu.rs` — main port source
- `xtal-macros/src/uniforms.rs` — bank naming scheme reference (a-x, 1-4)
- `sketches/src/sketches/templates/du_fs_template.wgsl` — reference shader
  structure (VertexInput, VertexOutput, Params, vs_main, fs_main,
  correct_aspect)
