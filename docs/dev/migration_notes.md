# xtal2 Code Review Notes

Review of `app.rs`, runtime files, and `frame_controller` as of 2026-02-19.
Updated to reflect the MIDI/map-mode additions in the latest revision.

## Summary

The structure is solid. The `XtalRuntime` flat struct, single
`on_runtime_event` dispatcher, and `ApplicationHandler` impl are all
well-organized. The render pipeline ordering is correct, the command/event
flow is clean, and the borrow discipline is sound. The notes below are
organizational improvements and a small number of behavioral gotchas — nothing
that breaks correctness today.

---

## `app.rs`

### 1. `render()` borrow scope is a structural constraint (minor)

The big inner block with `let (...) = { ... };` exists to release mutable
borrows on `self.context`, `self.graph`, etc. before doing post-submit work.
This is the right call, but `PendingPngCapture` holding a `wgpu::Buffer` means
GPU resource ownership is being passed across the borrow boundary. It works, but
the function will resist refactoring as the render path grows (e.g. multi-pass
captures). Not urgent — just keep in mind.

### 2. `resize()` is called redundantly on sketch switch and perf mode toggle

Both `switch_sketch()` and `set_perf_mode()` call `window.request_inner_size()`
and then call `self.resize()` directly. On most platforms `request_inner_size()`
triggers a `WindowEvent::Resized`, which calls `resize()` again — so the surface
gets reconfigured twice. Usually harmless, but inconsistent with the idiomatic
winit pattern. The standard approach is to let `Resized` drive `resize()`
exclusively, and only call it directly when you know the resize event won't fire
(e.g. when the window size does not change but the config needs updating).

### 3. `save_png_capture` GPU poll is safe but has a latent edge case

In the capture worker thread, `device.poll(WaitForSubmissionIndex(...))` is the
correct async readback pattern. One edge case: if the main window closes
immediately after a capture is queued, the `Arc<wgpu::Device>` clone in the
thread keeps the device alive and `poll()` will block until the submission
completes or the device is lost. In practice this is fine since the submission
has already been queued before the thread spawns, but it's worth knowing this
thread can outlive the event loop if a quit races with a capture.

### 4. `compute_row_padding` is duplicated

`compute_row_padding` is defined identically in both `app.rs` and
`frame_recorder.rs`. It should live in one place — probably a shared `gpu` or
`util` module — and be imported where needed.

### 5. `SketchUiState` clone-then-write-back pattern in `emit_web_view_load_sketch`

```rust
let mut sketch_state = self.current_sketch_ui_state(); // clone
if sketch_state.mappings.is_empty() {
    sketch_state.mappings = self.mappings_from_hub();
    self.current_sketch_ui_state_mut().mappings = sketch_state.mappings.clone();
}
```

This is a clone-then-write-back where a direct `_mut()` access would do the
same work with fewer allocations. Not a bug, just unnecessary cloning.

### 6. `on_runtime_event` returns `bool` to signal quit (implicit)

The `bool` return (true = quit was requested) is only used in
`process_commands()` and keyboard handlers. The intent is clear from comments,
but a named type or a two-variant enum would make it impossible to accidentally
ignore the early-exit semantics. Low priority.

### 7. `start_midi_clock_listener` does not cancel the previous listener

`ChangeMidiClockPort` calls `start_midi_clock_listener()` on every port change,
but there is no mechanism to stop the previously registered listener first.
If `midi::on_message` with `ConnectionType::GlobalStartStop` internally manages
a single slot this is fine — but if it fans out, repeated port changes could
leave stale listeners sending events. Worth verifying that the `midi` module's
`GlobalStartStop` connection type replaces rather than appends.

### 8. `normalize_midi_port_selections` silently falls back to port index 0

When the persisted port name is not found in the current port list, the runtime
falls back to `midi_input_ports[0]` and `midi_output_ports[0]`. This is
reasonable behavior, but it saves the corrected port to disk (`save_global_state`
is called if `midi_ports_updated`), which means a user who has a device
temporarily disconnected at startup will silently have their saved port overwritten.
Consider logging a more prominent warning or only saving on explicit user action.

---

## `frame_controller.rs`

### 9. Pacer accumulator is not reset on sketch switch

`switch_sketch()` calls `frame_controller::set_fps()` but does not call
`frame_controller::reset()`. If the new sketch has a different FPS and the pacer
has a large accumulator at switch time, the first organic tick after the switch
could produce multiple frame advances from stale debt accumulated at the old FPS.
In practice `switch_sketch` immediately calls `request_render_now()` which
bypasses the accumulator for the first frame, but subsequent ticks are still
affected. Consider calling `frame_controller::reset_timing(Instant::now())` (not
the full `reset()` which also zeroes the frame count) at the end of
`switch_sketch`.

---

## `recording.rs` / `frame_recorder.rs`

### 10. `bytes_per_pixel = 4` is hardcoded without a comment in `app.rs`

The still-image capture path in `app.rs` hardcodes `bytes_per_pixel = 4u32`.
This is correct for the formats that `recording_source_format()` can return
(`Bgra8*`, `Rgba8*`), but the assumption is implicit. A short comment tying it
to the format constraint would make it easier to catch if a new format is added
to the graph.

### 11. `USE_BLOCKING_MAP_WAIT = true` makes the non-blocking branch dead code

The `false` polling-with-sleep branch in `writer_thread_fn` is unreachable
because `USE_BLOCKING_MAP_WAIT` is a `const` set to `true`. Either delete the
non-blocking branch and the constant, or promote it to a proper feature flag if
you intend to support both paths.

### 12. `FinalizeMessage` enum has only one variant

`FinalizeMessage` could be a plain struct since only `Complete` exists. The enum
shell is noise unless more variants are anticipated.

---

## `events.rs`

### 13. `RuntimeCommand = RuntimeEvent` type alias conflates two concepts

Inbound commands and outbound events share the same type, with the distinction
living only in naming convention and channel direction. The silent-ignore arm in
`on_runtime_event` for `FrameSkipped | SketchSwitched(_) | Stopped | WebView(_)`
is the correct guard. Keep that catchall visible and documented — it is the only
thing preventing outbound-only events from being accidentally re-dispatched if
sent into the command channel.

---

## Priority Summary

| # | Severity | Item |
|---|----------|------|
| 2 | Low | Double `resize()` on sketch switch / perf mode toggle |
| 4 | Low | `compute_row_padding` duplicated in `app.rs` and `frame_recorder.rs` |
| 7 | Low | MIDI clock listener may not cancel previous listener on port change |
| 9 | Low | Pacer accumulator not reset on sketch switch (potential multi-frame advance) |
| 8 | Info | Normalize-port fallback silently overwrites persisted port selection |
| 11 | Cosmetic | `USE_BLOCKING_MAP_WAIT` + unreachable non-blocking branch |
| 12 | Cosmetic | `FinalizeMessage` single-variant enum |
| 5 | Cosmetic | Clone-then-write-back in `emit_web_view_load_sketch` |
| 6 | Cosmetic | `on_runtime_event` bool return is implicit |

Items 2 and 9 are the only ones with behavioral consequences worth acting on
before cutover. Everything else is structural or cosmetic.
