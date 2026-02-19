# xtal2 Migration: Runtime Parity Backlog

## Goal

Ship `xtal2` as the full runtime replacement for `xtal` with the same
user-facing behavior for:

- sketch switching at runtime
- ControlHub-driven animation/control scripts
- xtal-ui integration and event flow
- recording/performance tooling
- state/mapping persistence

## Hard Constraints

- Keep `nannou_osc`, remove all other Nannou runtime dependencies.
- No CPU drawing support.
- No per-sketch `bin` entrypoint pattern.
- Sketch asset lookup should use `SketchAssets` (no manifest boilerplate).

## Current Snapshot (2026-02-16)

### Foundation that is already in place

- Dynamic runtime registry with categorized sketches.
- Runtime sketch switching (`Box<dyn Sketch>` in the runner).
- `register_sketches!` and module-based sketch startup in `xtal2-sketches`.
- `SketchAssets::from_file(file!())` path flow.
- `FrameClock` and `WaitUntil` scheduling.
- Unified runtime event model in the runner (`RuntimeEvent` as source of truth,
  no internal `AppEvent` mirror).
- Runtime handler parity sweep started: `alert` / `alert_and_log` restored and
  randomize/snapshot flows aligned closer to xtal v1.
- One-way UI command flow enforced in runtime handlers (avoid rebroadcasting
  inbound UI commands as outbound UI events).
- OS-conventional directories restored (`config`/`cache` via `directories-next`,
  default images/videos/user-data via user folders + `Xtal` fallback).
- Runtime persistence scaffolding is now on disk (global settings and
  per-sketch controls/snapshots/mappings/exclusions).
- Sketch-state persistence root now defaults to the active sketch crate's
  `storage` directory (source-control-friendly):
  `storage/Controls/*` and `storage/global_settings.json`.
- ControlHub + shader/yaml hot reload integration.
- `web_view_process` is wired and launched by default via `run_registry`.
- `ax/ay/az/aw` var pattern is supported (legacy numeric aliases still parse).
- Runtime source layout cleanup is done (`xtal2/src` root is minimal).

### What is not complete yet

- UI event parity is still partial (several handlers remain scaffolds).
- Performance mode is not stateful in runtime and does not gate window
  resize/position behavior.
- MIDI/audio/map-mode/persistence paths are not ported to xtal2 runtime.
- Recording pipeline is not ported.
- Full behavioral parity tests against legacy runtime are missing.

## Active Phase Backlog

### Phase 1: UI bridge parity (critical)

The protocol enum is present, but command handling is incomplete.

#### 1.1 Incoming UI event coverage

Currently mapped to runtime commands:

- `Advance`
- `Paused(bool)`
- `Quit`
- `SwitchSketch(String)`
- `UpdateControlBool`
- `UpdateControlFloat`
- `UpdateControlString`

Still missing command mapping and runtime handling:

- [x] `PerfMode(bool)`
- [x] `ToggleFullScreen`
- [x] `ToggleMainFocus`
- [x] `Tap`
- [x] `TapTempoEnabled(bool)`
- [x] `TransitionTime(f32)`
- [x] `Randomize(Vec<String>)`
- [x] `Reset`
- [x] `Save(Vec<String>)`
- [x] `SnapshotStore(String)`
- [x] `SnapshotRecall(String)`
- [x] `SnapshotDelete(String)`
- [x] `MappingsEnabled(bool)`
- [x] `Mappings(...)` receive path
- [x] `CurrentlyMapping(String)`
- [x] `CommitMappings`
- [x] `RemoveMapping(String)`
- [x] `SendMidi`
- [x] `Hrcc(bool)`
- [x] `ChangeAudioDevice(String)`
- [x] `ChangeMidiClockPort(String)`
- [x] `ChangeMidiControlInputPort(String)`
- [x] `ChangeMidiControlOutputPort(String)`
- [x] `ChangeOscPort(u16)`
- [x] `ChangeDir(UserDir)` + `ReceiveDir(UserDir, String)`
- [x] `OpenOsDir(OsDir)`
- [x] `CaptureFrame`
- [x] `QueueRecord`
- [x] `StartRecording`
- [x] `StopRecording`
- [x] `ClearBuffer`

Status note: command routing is now complete; some handlers are still
stateful scaffolds pending full backend parity in phases 2/3.

#### 1.2 Outgoing runtime -> UI event coverage

Already emitted:

- `Init`
- `LoadSketch`
- `Paused`
- `HubPopulated`
- `UpdatedControls`
- `SnapshotSequenceEnabled`

Still missing (or not yet driven by real runtime state):

- [x] `AverageFps(f32)` (emitted once per second from runtime loop)
- [x] `Bpm(f32)` updates
- [x] `Mappings(...)`
- [x] `MappingsEnabled(bool)`
- [x] `CurrentlyMapping(String)`
- [x] `Encoding(bool)` (driven by real recorder start/finalize lifecycle)
- [x] `StartRecording` and `StopRecording` status events
- [x] `PerfMode(bool)` state echo/confirmation
- [x] Directory update confirmations (`ReceiveDir`) from persisted state path

#### 1.3 Performance mode parity (blocking issue)

- [x] Add runtime perf-mode state and `RuntimeCommand::SetPerfMode(bool)`.
  File touchpoints:
  `xtal2/src/runtime/events.rs`, `xtal2/src/runtime/web_view.rs`,
  `xtal2/src/runtime/app.rs`.
- [x] Stop hardcoding `LoadSketch.perf_mode = false`; emit actual runtime state.
  File touchpoint: `xtal2/src/runtime/app.rs`.
- [x] Gate main window resize/reposition on sketch switch when perf mode is on.
  File touchpoint: `xtal2/src/runtime/app.rs`.
- [x] Restore main-window size when leaving fullscreen/focus flows.
  File touchpoint: `xtal2/src/runtime/app.rs`.
- [ ] Ensure webview control window placement behavior respects perf mode without
  breaking control sizing expectations.
  File touchpoint: `xtal2/src/bin/web_view_process.rs`.

#### 1.4 UI bridge parity tests

- [x] Expand phase-1 tests beyond `SwitchSketch` to cover full mapping set.
- [x] Add one-way event-flow assertions so outbound UI events (`HubPopulated`,
  `UpdatedControls`, `SnapshotEnded`) are never remapped into inbound runtime
  commands.
- [x] Add parity assertions that bool/float/string UI updates map to one
  runtime update variant (`UpdateUiControl`).
- [x] Add callback guard test to ensure populated emissions are single-shot per
  population cycle.
- [ ] Add serialization/deserialization golden tests for all UI payload types.
- [ ] Add end-to-end smoke test for webview process message routing.

Exit criteria:

- xtal-ui actions drive runtime behavior equivalently to legacy xtal and all
  expected status events round-trip.

### Phase 2: MIDI, audio, map mode, and persistence

- [ ] Port map-mode runtime (`currently_mapping`, commit/remove, duplicate checks).
- [ ] Add v1-parity tests for map-mode event flow (commit/remove/send mappings).
- [ ] Port MIDI control in/out lifecycle and hrcc behavior.
- [ ] Port MIDI clock and OSC port runtime controls.
- [ ] Port audio device switching and runtime restarts.
- [x] Port global settings serialization/restore.
- [x] Port sketch state serialization/restore (snapshots, mappings, exclusions).
- [x] Hook persistence into runtime sketch-switch lifecycle (load on init/switch
  and save on explicit `Save` event).

Exit criteria:

- Mappings, device settings, snapshots, and sketch/global state persist and
  restore like legacy xtal.

### Phase 3: Recording and performance tooling

- [x] Port still frame capture flow (lossless PNG from GPU readback of graph present source).
- [x] Port queued recording, start/stop recording, and encode lifecycle.
- [x] Port frame recorder integration with fixed frame clock.
- [ ] Restore performance telemetry (`AverageFps`, dropped frame reporting).
- [ ] Verify recording correctness under pause/advance/switch scenarios.

Exit criteria:

- Recording and capture behavior match legacy runtime and remain frame-accurate.

### Phase 4: ControlHub parity hardening

- [x] Add parity guards for snapshot recall interpolation end-state.
- [x] Add parity guards for randomize(all/single) transition behavior.
- [x] Add parity guards for exclusions application in snapshot/randomize paths.
- [ ] Validate xtal2 YAML behavior against representative legacy scripts.
- [ ] Add parity fixtures that compare values over N frames between xtal and
  xtal2 for the same control script.
- [ ] Remove panic paths in hot code paths (for example remaining
  `unimplemented!()` branches) or convert them to explicit errors.
- [ ] Re-check snapshot and transition callback behavior against legacy UI flow.

Exit criteria:

- Same control scripts produce matching values and lifecycle events in xtal2 and
  xtal.

### Phase 5: Animation and timing parity

- [ ] Wire external transport timing sources fully (OSC/MIDI/hybrid runtime
  listeners).
- [ ] Port tap-tempo behavior and BPM update flow to runtime state.
- [ ] Validate timing reset semantics (MIDI start/continue/stop) against legacy
  behavior.

Exit criteria:

- Beat-synced animation and transport-driven timing match legacy behavior on
  reference sketches.

### Phase 6: var migration completion

- [ ] Add explicit deprecation warnings/tooling for numeric aliases (`a1..a4`).
- [ ] Set and document a removal milestone for numeric aliases.
- [ ] Keep docs/examples/templates fully on `ax/ay/az/aw` naming.

Exit criteria:

- `ax/ay/az/aw` is the only documented pattern and migration off numeric aliases
  is enforced by tooling.

### Phase 7: Cutover and cleanup

- [ ] Remove or archive obsolete legacy runtime code once parity is proven.
- [ ] Decide final crate naming (`xtal2` -> `xtal`) after parity sign-off.
- [ ] Add final migration notes for sketch authors and UI maintainers.

Exit criteria:

- xtal2 is the default and only runtime path for normal usage.

## Immediate Next-Sprint Order

1. Phase 2: port map-mode + persistence backend and tests first (highest
   leverage for runtime correctness).
2. Phase 1.3: finish perf-mode behavior parity for the webview control window.
3. Phase 1.2: finish remaining runtime status events (`AverageFps`,
   `Encoding`) with real backend state.
4. Phase 3: recording parity verification (capture-frame parity + pause/advance/switch validation).
5. Phase 4/5 parity fixtures and final behavior validation.
6. Phase 6 deprecation tooling and docs cleanup.
7. Phase 7 final cutover and cleanup.

## Locked API Decisions

1. Category registration shape is `{ title, enabled, sketches }`.
2. Category lives in registry registration, not `SketchConfig`.
3. Keep both flat `sketchNames` and structured catalog for UI compatibility.
4. `SketchAssets` is the standard sketch asset path mechanism.
5. `ax/ay/az/aw` is the forward var pattern.
