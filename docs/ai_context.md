# Xtal Project Context

## What This Project Is

Xtal is a Rust creative-coding framework (Nannou-based) for live generative
visual/audio performance with beat-synced motion, MIDI/OSC/audio control, and a
React-based control UI.

## Repo Map (Start Here)

- `/Users/lokua/code/xtal-project/xtal/src/framework/control/`
  - `control_hub.rs`: runtime control orchestration, snapshots, script updates
  - `config.rs`: YAML schema + deserialization + expression parsing helpers
- `/Users/lokua/code/xtal-project/xtal/src/framework/motion/`
  - `animation.rs`: beat-based animation primitives and timing behavior
- `/Users/lokua/code/xtal-project/sketches/src/sketches/`
  - sketches + their YAML control scripts (same folder as sketch `.rs`)
- `/Users/lokua/code/xtal-project/docs/`
  - `control_script_reference.md`: source of truth for script schema/behavior
  - `ui.md`: UI architecture and interaction notes
- `/Users/lokua/code/xtal-project/xtal-ui/`
  - React/TypeScript UI frontend

## High-Value Mental Model

- `ControlHub` is the runtime source of parameter truth.
- YAML control scripts define mappings and animations; they hot-reload.
- Sketches read values from the hub each frame (`hub.get("param")`).
- Timing-sensitive features should be reasoned about in beats, not wall-time.

## Hot Reload and Restart Boundaries

- Hot reload while app is running:
  - YAML control scripts
  - WGSL shaders
- Requires restart:
  - Rust code changes (`xtal`, `sketches`, `xtal-ui` backend integration)

This distinction saves debugging time when behavior appears “unchanged.”

## Fast Debug Workflow

1. Run sketch in release mode for realistic timing:
   - `RUST_LOG=xtal=info,sketches=info cargo run --release <sketch_name>`
2. If working on control scripts, edit YAML first to validate behavior quickly.
3. If behavior differs from expectations, check logs for YAML parse/validation
   errors before changing runtime logic.
4. For frontend changes, also verify UI build:
   - `npm --prefix /Users/lokua/code/xtal-project/xtal-ui run build`

## Creating a New Sketch

Use this when scaffolding a new sketch in `sketches/src/sketches/auto/`.

1. Pick a template and sketch name.
   - Example template: `sketches/src/sketches/templates/du_fs_template.*`
   - Example target: `auto_757`
2. Copy the template files into the auto module:
   - `auto_757.rs`
   - `auto_757.yaml`
   - `auto_757.wgsl`
3. Update the Rust sketch file:
   - Set `SKETCH_CONFIG.name` to the module name (e.g. `"auto_757"`).
   - Set `display_name` to the UI label you want.
   - Rename the sketch struct (`Template` -> `Auto757`).
   - Update YAML/WGSL paths in `ControlHub::from_path(...)` and
     `GpuState::new_fullscreen(...)` to the new filenames.
4. Register the sketch in module exports:
   - Add `pub mod auto_757;` to `sketches/src/sketches/auto/mod.rs`.
   - Add `pub use self::auto::auto_757;` to `sketches/src/sketches/mod.rs`.
5. Register it in `sketches/src/main.rs` inside `register!(...)` under AUTO.
6. Verify compile:
   - `cargo check -p sketches`

Control scripting and dynamic uniforms checklist:

- YAML hot-reloads at runtime via `ControlHub`.
- `ShaderParams::from((&wr, &hub))` maps script values into uniform banks.
- Use `var` in YAML to bind friendly control names to shader uniform slots (e.g.
  `var: a3`).
- Add direct per-frame overrides as needed (e.g.
  `params.set("a3", hub.animation.beats())`).

## Control Script Implementation Notes

- Prefer adding schema/validation in `config.rs` when possible so invalid input
  fails at parse/compile of config, not inside frame updates.
- Runtime update paths in `control_hub.rs` should stay simple and cheap; avoid
  recomputing static per-sequence/per-mapping facts each frame.
- Expression-backed booleans (`true`/`false` and string forms) are expected to
  work where config supports disabled-style flags.
- uniform a.w (a3 in rust/yaml) is beats - always use this in place of a
  traditional time uniform (it's already wired up)
- for periodic animations, use random_slewed, triangle, or round_robin - do not
  derive from beats in wgsl
- when adding rate or rate multiplier controls, make sure the `step` parameter
  is tempo friendly: divisions of 0.25 or 0.125 depending on the granularity you
  need

## UI Integration Pattern (When Exposing New Hub State)

When adding a new runtime status to UI:

1. Add a query method on `ControlHub` (single source of truth).
2. Thread field through web-view events (`LoadSketch` and incremental updates).
3. Store in UI app state.
4. Gate interactions in both:
   - UI components (visual affordance/overlay)
   - keyboard shortcuts/handlers (hard guard)

This avoids UI appearing disabled while hotkeys still mutate state.

## Common Pitfalls

- YAML numeric vs string coercion assumptions: serde will not coerce all scalar
  types automatically for typed fields.
- Beat-loop boundaries: clarify whether a terminal marker is an executable stage
  or just loop end.
- Overcomplicated runtime state: prefer deterministic recomputation from current
  beat + static config where feasible.

## Style and Collaboration Preferences

- Keep Rust code straightforward over clever.
- Avoid over-abstracting single-use runtime structures.
- Put validation where it prevents runtime surprises.
- If behavior choice is ambiguous, prefer semantic YAML that communicates intent
  clearly over compact but unclear forms.
- Ask the author directly when intent is ambiguous or multiple valid semantics
  exist; this is often faster and cheaper than deep exploration.
- If the likely answer is simple and local to current work, prefer a quick
  targeted check over broad tool-driven investigation.

## Useful References

- Control script docs:
  `/Users/lokua/code/xtal-project/docs/control_script_reference.md`
- UI architecture: `/Users/lokua/code/xtal-project/docs/ui.md`
- Framework entry area: `/Users/lokua/code/xtal-project/xtal/src/framework/`

## IMPORTANT! Coding Style Guidelines

1. Use consistent block indentation only; do not column-align fields or args.
2. Keep lines reasonably short (target ~80 chars when practical).
3. Group imports by std, external crates, and internal modules.
4. Use Rust naming conventions:
   - `snake_case` for functions/variables/modules
   - `PascalCase` for types/enums/traits
   - `SCREAMING_SNAKE_CASE` for constants
5. Prefer small, explicit functions and predictable control flow over clever
   abstractions.
6. Put comments above the code they explain; avoid end-of-line comments.
7. Keep frequently-read entry logic near the top; push helpers/utilities lower.

## Random Tidbits

- In general you don't need to edit \*\_controls.json files in the storage
  folder. They are autogenerated.
