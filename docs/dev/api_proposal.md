# Refined API Proposal

## Resource declarations return typed handles

```rust
let params = graph.uniforms();
let img    = graph.image(path);
let fb_a   = graph.texture2d();
let fb_b   = graph.texture2d();
```

No string IDs. Typos become compile errors. Resources are anonymous by default — the name you give the variable *is* the name.

---

## Render nodes terminate with their destination

```rust
graph.render()
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(img)
    .to(fb_a);               // offscreen texture

graph.render()
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(fb_a)
    .to_surface();           // final output
```

`.to()` / `.to_surface()` terminates the chain and registers the node. No separate `.write()` + `.add()`.

---

## Compute nodes terminate the same way

```rust
graph.compute()
    .shader(path)
    .read_write(field)
    .dispatch();             // terminates, registers node
```

---

## Feedback shorthand

```rust
let (ping, pong) = graph.feedback();

graph.render()
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(ping)
    .to(pong);

graph.present(ping);
```

`graph.feedback()` allocates the texture pair. Swap logic is just `.read(ping).to(pong)` and `.read(pong).to(ping)` — explicit, visible, no magic.

---

## Presentation is unchanged

```rust
graph.present(fb_a);    // blit to screen
// or just use .to_surface() on the last render node
```

---

## WGSL stays explicit

Bindings remain visible in shader code — the framework documents a stable, simple convention:

| Slot | Always |
|---|---|
| `@group(0) @binding(0)` | `params` uniform |
| `@group(1) @binding(0)` | sampler (if any reads) |
| `@group(1) @binding(1)` | 1st `.read()` texture |
| `@group(1) @binding(2)` | 2nd `.read()` texture |
| `@group(1) @binding(N)` | Nth `.read()` texture |

No generated bindings, no magic — but the convention is fixed, documented, and consistent everywhere. The framework can also emit the required WGSL struct/binding declarations on error to eliminate the lookup entirely.

---

## What this doesn't change

- WGSL shader authoring is unchanged
- Mesh system is unchanged (separate concern)
- `SketchAssets`, `SketchConfig`, `Sketch` trait are unchanged
- `FullscreenShaderSketch` convenience wrapper still makes sense for the simple case

---

## Full example (feedback sketch)

**Before:**
```rust
graph.uniforms("params");
graph.texture2d("feedback_a");
graph.texture2d("feedback_b");

graph.render("step_a")
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read("params")
    .read("feedback_a")
    .write("feedback_b")
    .add();

graph.render("step_b")
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read("params")
    .read("feedback_b")
    .write("feedback_a")
    .add();

graph.present("feedback_a");
```

**After:**
```rust
let params        = graph.uniforms();
let (ping, pong)  = graph.feedback();

graph.render()
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(ping)
    .to(pong);

graph.render()
    .shader(path)
    .mesh(Mesh::fullscreen_quad())
    .read(params)
    .read(pong)
    .to(ping);

graph.present(ping);
```
