> NOTE: this document for now will serve as a dumping ground until I figure out
> exactly how I want to organize more formal documentation

# Table of Contents

- [Sketch Boilerplate](#sketch-boilerplate)
  - [Fullscreen Shader Template](#fullscreen-shader-template)
  - [Custom Sketch Template](#custom-sketch-template)
- [Controls](#controls)
- [Animation](#animation)
- [Control Scripting](#control-scripting)
- [User Interface](#user-interface)
- [Audio](#audio)
  - [Multichannel Audio](#multichannel-audio)
    - [Aggregate Device Setup](#aggregate-device-setup)
  - [Single Channel, Multiband Audio (experimental)](#single-channel-multiband-audio-experimental)
    - [Aggregate Device Setup](#aggregate-device-setup-1)
    - [Routing Audio to Blackhole 2ch Out(3/4):In(1/2)](#routing-audio-to-blackhole-2ch-out34in12)
- [MIDI](#midi)
  - [Loopback (Ableton)](#loopback-ableton)
  - [Sync Recordings](#sync-recordings)
- [Recording Performance Flags](#recording-performance-flags)
- [Open Sound Control (OSC)](#open-sound-control-osc)
  - [L.OscTransport](#losctransport)
  - [L.OscSend](#loscsend)
- [Timing](#timing)
- [Running Multiple Instances](#running-multiple-instances)
- [Tips](#tips)
- [General Resources](#general-resources)

# Sketch Boilerplate

Xtal sketches expose a `SketchConfig` and return a type implementing `Sketch`.
For most shader-driven sketches, controls are declared in YAML and loaded via
`with_control_script(...)`.

### Fullscreen Shader Template

```rust
use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "basic",
    display_name: "Basic",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 1920,
    h: 1080,
    banks: 4,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
```

### Custom Sketch Template

```rust
use std::path::PathBuf;
use xtal::prelude::*;

pub struct MySketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for MySketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}
```

# Controls

Controls are defined in YAML. The runtime handles control evaluation,
randomization, snapshots, mappings, and hot-reload internally.

**Example (hue/saturation/lightness setup):**

```yaml
hue:
  type: slider
  var: aw
  range: [0.0, 1.0]
  default: 0.0

saturation:
  type: slider
  var: bx
  range: [0.0, 1.0]
  default: 0.0

lightness:
  type: slider
  var: by
  range: [0.0, 1.0]
  default: 0.0
```

In WGSL, read values from uniform banks:

```wgsl
let hue = params.a.w;
let saturation = params.b.x;
let lightness = params.b.y;
```

Runtime-managed uniform slots are:

- `ax`: window width
- `ay`: window height
- `az`: current beat time

Use `aw` onward for your own mapped control values.

# Animation

Animation mappings are also declared in YAML.

```yaml
hue_anim:
  type: triangle
  var: aw
  beats: 16.0
  range: [0.0, 1.0]
```

This creates a ping-pong ramp over musical beats. Because animation timing is
beat-based, changing BPM scales animation rate automatically.

# Control Scripting

Control scripts are designed for iterative workflow: edit YAML and the active
control graph updates at runtime.

For full schema docs, see [docs/control_script_reference.md](./control_script_reference.md).

Good starting points in this repo:

- [basic template yaml](../sketches/src/templates/basic.yaml)
- [animation dev yaml](../sketches/src/dev/animation_dev.yaml)

# User Interface

In the bottom of the UI is a console window that displays system alerts and
operation feedback. Help mode displays guidance and keyboard shortcuts for the
currently hovered control.

Keyboard shortcuts are documented in [docs/ui.md](./ui.md).

# Audio

## Multichannel Audio

**Example**

```yaml
bass_drum:
  type: audio
  channel: 0
  slew: [0.0, 0.0]
  pre: 0.0
  detect: 0.0
  range: [0.0, 1.0]

snare_drum:
  type: audio
  channel: 1
  slew: [0.65, 0.65]
  pre: 0.0
  detect: 0.0
  range: [0.0, 1.0]
```

Audio controls treat each channel as a control signal with optional smoothing.
You can configure the audio device globally in **Settings > Audio**.

### Aggregate Device Setup

![Mac Aggregate Device Setup](../assets/aggregate-device-multichannel.png)

> In the above setup I use 1-2 as the main outs and send the multichannel data
> out to channels 3-18 in my DAW which then appear on Blackhole channels 1-16

## Single Channel, Multiband Audio (_experimental_)

The `Audio` helper in `xtal::io::audio` processes a single input channel and can
be useful for experimental FFT-based responses.

### Aggregate Device Setup

![Mac Aggregate Device Setup](../assets/aggregate-device-setup.png)

### Routing Audio to Blackhole 2ch `Out(3/4):In(1/2)`

> Note that Blackhole automatically routes whatever its output channels are to
> its own input, so sending audio out to Blackhole 3/4 will automatically appear
> on inputs 1/2 in this setup; you don't even need to configure the inputs in
> Ableton at all for this to work (just as long as you have the output config
> set to "Xtal" and enable the appropriate outputs in the output config under
> Live's audio preferences)

![Ableton Live - Blackhole Track Routing](../assets/live-blackhole-track-routing.png)

# MIDI

**Example**

```yaml
foo:
  type: midi
  channel: 0
  cc: 0
  range: [100.0, 500.0]
  default: 0.0

bar:
  type: midi
  channel: 0
  cc: 1
  range: [0.0, 1.0]
  default: 0.0

baz:
  type: midi
  channel: 0
  cc: 2
  range: [0.0, 1.0]
  default: 0.0
```

MIDI clock, input, and output ports can be set in **Settings > MIDI**.

### Loopback (Ableton)

To automate synth parameters in Ableton and Xtal parameters simultaneously from
_the same UI CC control in Live_ (as opposed to a physical control, in which
case you can skip this section), you need to enable MIDI loopback by sending
MIDI to `Xtal In` and also route `Xtal In` back in to Live to control
parameters. Here's the routing:

![Live MIDI Preferences](../assets/live-midi-prefs.png)

To use Ableton automation lanes to control Xtal params, follow these steps:

1. Create a MIDI track and clip and add CC automation to it.
2. In the tracks **MIDI To** router, select `IAC Driver Xtal In` and `Ch. 1`.

Those steps are all you need to send MIDI to Xtal to control parameters. As for
controlling a live parameter with that same CC, follow these steps:

1. Play your clip containing the CC data.
2. Stop the transport (this is important).
3. Enter MIDI Mapping mode.
4. Locate the parameter you want to map and select it (make sure it's the last
   thing you've clicked).
5. Press the Space bar to start the transport.

> Note: the above instructions are for working without a MIDI controller. When
> working with a MIDI controller you can map the MIDI control to an Ableton
> device knob that sends CC out to Xtal and also map the controller to an
> Ableton parameter. In this case _you do not_ want Xtal enabled in Ableton's
> MIDI Input ports at all as that complicates routing.

### Sync Recordings

With MIDI ports configured in your DAW to send clock to Xtal, you can sync video
recordings with DAW audio.

1. In Ableton > Preferences > Record, make sure **Start Transport With Record**
   is set to **Off**.
2. Hit **Q Rec** in Xtal.
3. Arm tracks in Ableton and arm transport recording.
4. Press Play in Ableton. Xtal recording should start/stop with transport.

# Recording Performance Flags

Xtal's ffmpeg recorder reads these environment variables at startup:

- `XTAL_RECORDING_PRESET`: Sets the `libx264` preset. Default: `veryfast`.
  Available presets (fastest to slowest): `ultrafast`, `superfast`, `veryfast`,
  `faster`, `fast`, `medium`, `slow`, `slower`, `veryslow`, `placebo`.
- `XTAL_RECORDING_NUM_BUFFERS`: Number of GPU readback buffers in the capture
  ring. Default: `6`, minimum effective value: `2`.

Examples:

```bash
# Use higher quality compression at runtime cost.
XTAL_RECORDING_PRESET=fast just start basic

# Prioritize capture throughput with more buffering.
XTAL_RECORDING_PRESET=ultrafast XTAL_RECORDING_NUM_BUFFERS=8 just start basic
```

# Open Sound Control (OSC)

Xtal supports OSC and includes two MaxForLive devices for Ableton integration.

**Example**

```yaml
foo:
  type: osc
  range: [100.0, 500.0]
  default: 22.0

bar:
  type: osc
  range: [0.0, 1.0]
  default: 22.0

baz_qux:
  type: osc
  range: [0.0, 1.0]
  default: 22.0
```

For OSC controls, the YAML mapping name is used as the OSC address (runtime
handles the leading slash).

### L.OscTransport

[assets/L.OscTransport.amxd][osc-transport]

![L.OscTransport MaxForLive Device](../assets/osc-transport.png)

Place this on any track in Ableton and it will send high precision clock and
transport location to Xtal.

### L.OscSend

[assets/L.OscSend.amxd][osc-send]

![L.OscSend MaxForLive Device](../assets/osc-send.png)

A basic OSC value sender. Some Ableton OSC tools send low-resolution data;
these devices are intended for high-resolution control.

# Timing

Timing mode is configured per sketch by overriding `timing_mode()`:

```rust
fn timing_mode(&self) -> TimingMode {
    TimingMode::Osc
}
```

Available timing modes are:

- `TimingMode::Frame` (default)
- `TimingMode::Osc`
- `TimingMode::Midi`
- `TimingMode::Hybrid`
- `TimingMode::Manual`

# Running Multiple Instances

To run multiple Xtal instances simultaneously (e.g., two different sketches each
with their own UI window), set `XTAL_UI_PORT` to give each instance a unique
port for the Vite dev server and WebView connection:

```bash
# Terminal 1: default port (3000)
just start sketch_a

# Terminal 2: custom port
XTAL_UI_PORT=3001 just start sketch_b
```

You'll also need to run a separate Vite dev server for each port:

```bash
# Terminal A: default port
bun --cwd xtal-ui start

# Terminal B: custom port
XTAL_UI_PORT=3001 bun --cwd xtal-ui start -- --port 3001
```

The default port is `3000` when `XTAL_UI_PORT` is not set.

# Tips

- Keep `ax`, `ay`, `az` reserved for runtime uniforms (resolution + beat).
- Prefer stable, descriptive YAML names for controls, especially OSC controls
  where names double as addresses.
- If a YAML edit is invalid, the previous valid control state continues running.
- Use [docs/ui.md](./ui.md) for updated shortcut and panel behavior.

# General Resources

- https://sotrh.github.io/learn-wgpu
- https://inconvergent.net/generative/
- http://www.complexification.net/
- http://www.complexification.net/gallery/machines/sandstroke/
- https://n-e-r-v-o-u-s.com/projects/albums/floraform-system/
- https://www.andylomas.com/cellularFormImages.html
- https://thebookofshaders.com/
- https://github.com/jasonwebb/2d-space-colonization-experiments
- https://paulbourke.net/geometry/
- https://easings.net/

[blackhole]: https://existential.audio/blackhole/
[coding-train]: https://thecodingtrain.com/
[ffmpeg]: https://ffmpeg.org/
[insta]: https://www.instagram.com/lokua/
[just]: https://github.com/casey/just
[livemtc]: https://support.showsync.com/sync-tools/livemtc/introduction
[xtal-sketches]: ../sketches/src
[osc-send]: ../assets/L.OscSend.amxd
[osc-transport]: ../assets/L.OscTransport.amxd
[p5]: https://github.com/Lokua/p5/tree/main
[tao]: https://github.com/tauri-apps/tao
[vite]: https://vite.dev/
[webview]: https://en.wikipedia.org/wiki/WebView
[wry]: https://github.com/tauri-apps/wry
