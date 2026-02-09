# Xtal Recording Refactor Summary (for External Code Review)

## Goal

Replace Nannou `capture_frame` (PNG-per-frame + post-encode) with a custom
realtime recorder that:

- Captures every rendered frame.
- Preserves beat/audio sync by blocking under backpressure (instead of
  dropping).
- Streams raw RGBA frames directly to ffmpeg via stdin.

---

## Original Problem

`nannou::capture_frame` caused severe frame drops on complex sketches due to PNG
encoding workload and internal capture thread contention.

---

## Major Refactor Already Implemented (before this review pass)

Core architecture moved to custom capture pipeline:

- New runtime module: `xtal/src/runtime/frame_recorder.rs`
- Integration updates in:
  - `xtal/src/runtime/recording.rs`
  - `xtal/src/runtime/app.rs`
  - `xtal/src/runtime/mod.rs`

Flow:

1. In `view` while `Frame` is alive:
   - Ensure recorder GPU resources exist (`TextureReshaper`, dst texture,
     readback buffers).
2. After frame submit/drop:
   - Encode reshape + texture-to-buffer copy.
   - Submit GPU commands.
   - Queue buffer for writer thread.
3. Writer thread:
   - Map readback buffer.
   - Write raw RGBA to ffmpeg stdin.
   - Return buffer index to main thread.
4. On stop:
   - Send stop signal.
   - Drain/join writer.
   - Wait for ffmpeg exit.

---

## Additional Changes Applied in This Iteration

File touched: `xtal/src/runtime/frame_recorder.rs`

### 1) Backpressure behavior aligned with sync model

- Changed buffer acquisition and writer queue handoff from drop-prone behavior
  to blocking behavior.
- Removed frame-drop behavior when writer queue is full.
- Added warning logs when waiting for free readback buffers exceeds frame
  budget.

Why:

- Matches requirement: capture every rendered frame and preserve audio/beat
  sync.

### 2) Reduced pipe write overhead for padded rows

- Previously: padded-row path wrote each row to ffmpeg separately (many small
  writes/frame).
- Now: copy padded rows into one contiguous frame buffer and issue a single
  `write_all`.

Why:

- Significantly reduces syscall overhead and pipe fragmentation pressure.

### 3) Avoided ffmpeg stderr backpressure stalls

- ffmpeg now runs with:
  - `-hide_banner`
  - `-loglevel error`
  - `-nostats`
- ffmpeg stderr changed from piped to null.

Why:

- Prevents potential deadlock/stall when stderr is piped but not continuously
  drained.

### 4) Tunable capture/encode settings via env vars

Added/updated runtime flags:

- `XTAL_RECORDING_PRESET`
  - Default changed to `veryfast`.
  - Passed through to `libx264 -preset`.
- `XTAL_RECORDING_NUM_BUFFERS`
  - New configurable readback ring size.
  - Default: `6`, minimum `2`.

Why:

- Gives operational control over realtime throughput vs compression efficiency.

### 5) Writer polling behavior softened

- Replaced long blocking `device.poll(Maintain::Wait)` usage in writer mapping
  loop with polling loop using `Maintain::Poll` + short sleep.

Why:

- Intended to reduce prolonged lock contention with render-thread GPU work.

---

## Docs Updated

File: `docs/docs.md`

Added/expanded **Recording Performance Flags** section:

- Preset list and practical usage guidance.
- Clarified compression/CPU tradeoff (slower preset = more CPU, typically better
  compression).
- Added rule of thumb:
  - Use the slowest preset that still sustains realtime without persistent wait
    warnings.
- Explained `XTAL_RECORDING_NUM_BUFFERS` purpose/tradeoffs and 1080p memory
  estimates.
- Added shell examples.

---

## Current Practical Findings (from manual testing in this thread)

- At higher resolution (`~2336x1314`), `medium` preset could not sustain
  realtime.
- Switching to `XTAL_RECORDING_PRESET=veryfast` materially improved behavior.
- At `1920x1080`, recording is reported as performing well.

---

## Known/Expected Tradeoffs

- GPU readback bandwidth cost is still paid regardless of live-vs-offline
  encoding strategy.
- Live ffmpeg piping avoids massive raw-frame disk I/O and extra post-pass read,
  but adds encode CPU load during capture.
- Increasing `NUM_BUFFERS` smooths bursts but does not solve sustained encoder
  throughput deficits.

---

## Suggested Reviewer Focus

1. Confirm no hidden frame-drop paths remain in recorder hot path.
2. Evaluate whether writer thread mapping/poll strategy can be further
   optimized.
3. Assess whether reshaper/copy submission can be merged more tightly with
   existing frame submission flow.
4. Verify stop/finalization robustness under rapid start/stop cycles.
5. Validate memory/perf behavior at `1920x1080@60` vs larger capture sizes.
6. Consider optional hardware encoder path for realtime headroom on supported
   platforms.

---

## Verification Performed in This Iteration

- `cargo check -q` ran successfully after recorder changes.
