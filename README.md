# lattice

A framework build around [Nannou][nannou] with a feature-packed control UI.

## Intro

Lattice is a hybrid Rust application and library build on top of the
[Nannou](nannou) creative coding framework. It is essentially one big Nannou app
packed with tons of features to aid in live performance with a strong emphasis
on interaction and musically-aware synchronization.

If you are already familiar with Rust and Nannou you should have no problem
getting Lattice up and running quickly. If you are not familiar with Rust or
Nannou but have experience with creative coding then I highly recommend you get
comfortable building Nannou sketches first, starting with
[their guide](https://guide.nannou.cc/). If you are completely new to creative
coding I highly recommend checking out [The Coding Train](coding-train). All
documentation in this project assumes you have a foundational understanding of
Rust and Nannou.

## Features

- **Runtime switching of sketches**
- **Video recording** with one button press (requires [ffmpeg][ffmpeg])
- **Beat-based animation** with musical timing
- **Flexible synchronization** - BPM, MIDI clock, MIDI Time Code, or Ableton
- **Parameter automation** via MIDI CC, OSC, and audio input
- **Recording sync** with MIDI Start for perfect post-production alignment
- **Hot-reloadable controls** via YAML files (see
  [Control Scripting](#control-scripting))
- **Simple UI controls** - Add sliders, checkboxes, and selects with minimal
  code
- **Persistent parameters** automatically saved per sketch
- **Hot-reloadable WGSL shaders** with starter templates
- **Snapshots system** - Store/recall settings with musical transitions
- **Parameter randomization** with configurable transition times
- **Selective control exclusions** from randomization
- **MIDI Learn** - Map hardware controllers to any UI parameter
- **Tap Tempo** for syncing with live music
- **Adaptive theming** for light/dark mode

### Light Mode

![Lattice Controls - Light Theme](assets/ui-light.png)

### Dark Mode

![Lattice Controls - Dark Theme](assets/ui-dark.png)

## Getting Started

> **Note:** Lattice is pre-v1 and transitioning from an application to a
> reusable library. Currently developed on macOS and requires running in "dev
> mode." Cross-platform compatibility expected but not fully tested.

## Getting Started

> **Note:** Lattice is pre-v1 and transitioning from an application to a
> reusable library. Currently developed on macOS and requires running in "dev
> mode." Cross-platform compatibility expected but not fully tested.

**Requirements:**

- [Rust][rust]
- [Node][node] or [Bun][bun] (preferred) for the UI
- (Optional) [ffmpeg][ffmpeg] for video exports

**Quick Start:**

1. Clone or fork this repo. Until Lattice has a proper release on crates.io, you
   must use [lattice-sketches] app (alternatively you can create your own
   workspace folder and follow the same pattern – this will make it easier to
   merge in upstream changes without interfering with your own code).
2. [lattice-sketches] is my personal sketch project inlined here for development
   convenience - you can see screenshots in the auto-generated
   [markdown index](index.md) or snippets of my audio-visual compositions on
   [Instagram][insta]. Feel free to use these examples or start from scratch.
3. Start the UI server:
   ```sh
   cd ./lattice-ui
   bun install  # or npm install (first time only)
   bun start    # or npm start
   ```
4. In another terminal, run the main app:
   ```sh
   cargo run --release  # optionally add <sketch> to specify which loads
   ```

For full documentation, run `cargo doc --package lattice --open` in the project
root.

[blackhole]: https://existential.audio/blackhole/
[bun]: https://bun.sh/
[coding-train]: https://thecodingtrain.com/
[config]: src/config.rs
[control-script-test]: src/sketches/scratch/control_script_test.rs
[ffmpeg]: https://ffmpeg.org/
[insta]: https://www.instagram.com/lokua/
[just]: https://github.com/casey/just
[lattice-sketches]: lattice-sketches/sketches
[midi-sketch]: src/sketches/midi_test.rs
[node]: https://nodejs.org/en
[nannou]: https://github.com/nannou-org/nannou
[osc-send]: assets/L.OscSend.amxd
[osc-transport]: assets/L.OscTransport.amxd
[p5]: https://github.com/Lokua/p5/tree/main
[rust]: https://www.rust-lang.org/
[template]: src/sketches/templates/template.rs
[tao]: https://github.com/tauri-apps/tao
[vite]: https://vite.dev/
[webview]: https://en.wikipedia.org/wiki/WebView
[wry]: https://github.com/tauri-apps/wry
