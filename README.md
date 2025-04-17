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
- **Record video** with a press of a button (requires [ffmpeg](ffmpeg) on your
  PATH)
- **Advanced Animation** with times specified in musical beats
- Sync animations to BPM and frame count, MIDI clock, MIDI Time Code, or an
  Ableton OscTransport plugin for rock solid timing
- Automate parameters with external **MIDI CC, OSC, and Multichannel Audio**
  with peak, rms, and multiband mechanisms all available through a dead simple
  API
- Sync sketch recording with external MIDI Start message which makes it very
  easy to align your track with the visuals perfectly in post-production
- Write animations in code or configure your sketch to use an external yaml file
  that can be hot-reloaded at runtime (similar to live coding - see
  [Control Scripting](#control-scripting))
- Declarative **per-sketch UI controls** definitions to easily add sliders,
  selects, and checkboxes for sketch parameter control
- **Automatic store/recall** of per-sketch UI controls/parameters that can be
  source controlled
- **Hot reloadable WGSL shaders** with various templates to simplify setup
- **Snapshots** - store and recall all GUI, MIDI, and OSC controls in the UI's
  Snapshot Editor or by pressing `Shift + Number` to save and
  `<PlatformModifier> + Number` to recall. Snapshots are interpolated to/from at
  a configurable musical length from 1/16th note up to 4bars. Great for live
  performance!
- **Randomization** - randomize all controls with configurable transition time
  (_amazing_). Clicking on a slider's label will randomize just that single
  parameter, while `<PlatformMod> + Click` on a label will revert it to its last
  saved state. This coupled with Snapshots will have you playing your sketch
  like a musical instrument.
- **Exclusions** - a column of checkboxes that pops up to the left of each
  control allowing you to exclude it from **Randomization**, saved with the
  sketch.
- Runtime mappings of MIDI CC to UI sliders, AKA **MIDI Learn**, saved with the
  sketch.
- Ability to override sketch BPM via **Tap Tempo** to sync with music during
  live performance
- UI adapts to your operating system's theme preference (see screenshots below)

### Light Mode

![Lattice Controls - Light Theme](assets/ui-light.png)

### Dark Mode

![Lattice Controls - Dark Theme](assets/ui-dark.png)

## Getting Started

> DISCLAIMER: Lattice is still pre-v1 and is currently in transition from a
> binary application meant to be cloned into a reusable library. It has been
> developed on macOS and has yet to be tested on other systems, though I assume
> it'll run just fine being that its dependencies are cross-platform. At this
> time it has no production build and must be run in "dev mode" – see below

Some software you'll need:

- Rust
- Node or Bun for running the UI (for now at least until bundling is
  implemented)
- (optional) [ffmpeg](todo-link) available on your path for video exports

Until Lattice has a proper release on crates.io, you must clone this repo and
run the [lattice-sketches] app. This is my personal sketch project inlined here
for development convenience (You can see screenshots here on GitHub by looking
at the auto generated [markdown index](index.md) or checkout some snippets of my
audio-visual compositions on [Instagram][insta]) – feel free to use these to get
started or just delete them and start from scratch.

You will need to run two separate terminal processes: one for the UI controls (A
Typescript/React app rendered in a [WebView][webview] with [Tao][tao] and
[Wry][wry], served with [Vite][vite]) and another for the Rust backend.

1. Launch the frontend app server

```sh
cd ./ui
bun start # or npm start
```

In another terminal window, launch the main Lattice app:

```sh
cargo run --release -- <sketch>
```

At this point you should see a main window with a template sketch and a UI. If
not – please file an issue!

For comprehensive documentation (until we have a published version) run
`cargo doc --package lattice --open` in the project root.

# Lattice & Nannou

As mentioned in the intro, Lattice is essentially one big Nannou app. The first
major difference is that a Lattice sketch must export a `SketchConfig` const
containing metadata needed for the runtime to properly boot a sketch. The second
major difference is that instead of the standalone `model`, `update`, and `view`
functions as you find in raw-Nannou, a Lattice sketch must provide an
implementation of the `Sketch` trait. You may also notice a 3rd context argument
in each method not found in the Nannou signatures – we'll get into that later –
but besides these differences, everything is the same as a Nannou app and Nannou
is still the crate you're likely to interact with the most in your code.

### Nannou Boilerplate

```rust
use nannou::prelude::*;

struct Model {}

fn model(app: &App) -> Model {
    Model {}
}

fn update(app: &App, model: &mut Model, update: Update) {
    // update model data
}

// optional
fn event(app: &App, model: &mut Model, event: Event) {
    // respond to window and keyboard events
}

fn view(app: &App, model: &Model, frame: Frame) {
    // draw stuff
}
```

### Lattice Boilerplate

```rust
use lattice::prelude::*;
use nannou::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "MySketch",
    display_name: "My Sketch",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
};

pub struct MySketch {}

pub fn init(app: &App, ctx: &Context) -> MySketch {
    Model {}
}

impl Sketch for MySketch {
    fn update(&mut self, app: &App, update: Update, ctx: &Context) {
        // update model data
    }

    // optional
    fn event(&mut self, app: &App, event: &Event, ctx: &Context) {
        // respond to window and keyboard events
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        // draw stuff
    }
}
```

At this point there is nothing really telling about _why_ one might want to use
Lattice over Nannou, so let's get into some of the benefits in the next
section...

## ControlHub

At the heart of Lattice is the `ControlHub` struct (which we'll refer to as hub
from hereon). The hub is the one-stop shop for all controls and animations used
in a sketch.

```rust
#[derive(SketchComponents)]
pub struct MyModel {
    hub: ControlHub<Timing>
}
```

The above example shows the two requirements needed to use the hub:

1. The model must derive `SketchComponents`. This provides implementations
   needed for the runtime to communicate with the hub
2. a `hub` field placed directly on the Model. The field can also be named
   `controls` if you prefer, but it has to be either `hub` or `controls`.

Now let's use the hub:

```rust
#[derive(SketchComponents)]
pub struct Example {
    hub: ControlHub<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> Example {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider_n("hue", 0.0)
        .slider_n("saturation", 0.0)
        .slider_n("lightness", 0.0)
        .build();

    Example { hub }
}

impl Sketch for Example {
    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let draw = app.draw();

        draw.background.color(WHITE);

        let color = hsl(
            self.hub.get("hue"),
            self.hub.get("saturation"),
            self.hub.get("lightness")
        );

        draw.ellipse()
            .color(hsl())
            .radius(200.0)
            .x_y(0.0, 0.0);

        draw.to_frame(app, &frame).unwrap();
    }
}
```

This sketch renders a circle in the middle of the screen and let's you change
its color. If you adjust the sliders then press the **Save** button, the values
of those sliders will be recalled the next time you run the sketch. If you click
the label of the slider component, it will move to a random value over the
transition time set by the **Transition Time** dropdown (expressed in musical
beats). If you press the **Randomize** button, it will randomize all three
sliders! If you don't like the changes, you can press the **Reload** button to
revert the sketch to its last saved state (or the defaults you set in your
sketch if you haven't yet saved). If you like the changes but don't want them to
be the defaults that show when you first load the sketch, you can press the
**Snapshots** button and save a snapshot to any 1 out of 10 slots for later
recall. Now let's imagine that while you enjoy randomizing all the sliders,
you'd prefer that the `hue` slider remained fixed at 10.33; for that you can
press the **Exclusions** button which will allow you to exclude any control from
global randomization. Of course this is all only so interesting when you're
simply changing the colors of a single circle, but allow yourself a moment to
imagine the creative possibilities with a more complex sketch with 10 or 20
controls. Hopefully this now gives you a better idea of what Lattice provides on
top of Nannou.

Here's a kitchen sync example you can use for reference:

```rust
let hub = ControlHubBuilder::new()
    .timing(Timing::new(ctx.bpm()))
    // name, default_value, (min, max), step, disabled_fn
    .slider("a", 50.0, (0.0, 100.0), 1.0, None)
    // slider "normalized" – name, default_value
    .slider_n("b", 0.0)
    // name, default_value, options, disabled_fn
    .select("stuff", "a", &["a", "b", "c"], None)
    // name, default_value, options, disabled_fn
    .checkbox("d", false, None)
    // Adds a faint horizontal line between controls
    .separator()
    // name, (channel, controller), (min, max), default_value
    .midi("e", (0, 0), (0.0, 1.0), 0.5)
    // midi "normalized"
    .midi_n("f", (0, 1), 0.5)
    // address, range, default_value
    .osc("f", (0.0, 100.0), 0.5)
    // osc "normalized"
    .osc_n("g", 0.5)
    .audio(
        "h",
        AudioControlConfig {
            channel: 0,
            slew_limiter: SlewLimiter::default(),
            pre_emphasis: 0.0,
            detect: 0.0,
            range: (0.0, 1.0),
            value: 0.0,
        },
    )
    .build();
```

## Animation

Building on the ControlHub example sketch, let's add some animation. Instead of
using a slider to control hue, let's animate it over time:

```rust
let hue = self.hub.animation.tri(16.0);

let color = hsl(
    hue,
    self.hub.get("saturation"),
    self.hub.get("lightness")
);
```

The `Animation::tri` method generates a linear ramp from 0.0 to 1.0 and back to
0.0 over the time expressed in its `duration` argument. In this case that
animation will last for 16 beats, or 4 bars. The tempo being used is what you
defined in your sketch's `SketchConfig::bpm` field, however you can override
this at runtime by using the **Tap Tempo** button. If you are not familiar with
musical timing here's the TL;DR: set your `bpm` to 60.0 – this means 1.0 beat
will last exactly 1 second. If you want your animation to last 10 seconds, use
10.0. That's basically it! But unlike using raw time units like seconds, these
times will scale relative to `bpm`, so if you now set you're `bpm` to 120.0,
everything will run twice as fast and you didn't need to update any code to
accomplish this! Not to mention you can just Tap Tempo to synch with your DJ
homey on stage.

This is just the tip of what the Animation module is capable of; consult the
[docs][todo] for more details.

## Control Scripting

While Lattice's various control and animation methods are easy to setup, it's a
bit of pain to have to restart the rust sketch every time you want to change an
animation or control configuration – especially as your sketch matures. For this
reason Lattice provides a script-like mechanism that uses yaml for configuration
and adds these controls dynamically and self-updates at runtime when the yaml
file is changed. You still have to take care to setup the routings in your
sketch (e.g. `let radius = self.hub.get("radius")`), but once these routings are
in place you are free to edit their ranges, values, timing, etc. It's also worth
knowing that Control Scripting makes certain things like disabling controls
based on the values of other controls and parameter modulation much easier than
they'd be in real code. Checkout any sketch in
[lattice-sketches][lattice-sketches] that has a corresponding yaml file of the
same name for a working example or
[docs/control_script_reference.md](docs/control_script_reference.md) for
comprehensive documentation.

[blackhole]: https://existential.audio/blackhole/
[breakpoints]:
  https://media.githubusercontent.com/media/Lokua/lattice/main/images/breakpoints-flin7.png
[coding-train]: https://thecodingtrain.com/
[config]: src/config.rs
[control-script-test]: src/sketches/scratch/control_script_test.rs
[ffmpeg]: https://ffmpeg.org/
[insta]: https://www.instagram.com/lokua/
[just]: https://github.com/casey/just
[lattice-sketches]: lattice-sketches/sketches
[midi-sketch]: src/sketches/midi_test.rs
[nannou]: https://github.com/nannou-org/nannou
[osc-send]: assets/L.OscSend.amxd
[osc-transport]: assets/L.OscTransport.amxd
[p5]: https://github.com/Lokua/p5/tree/main
[template]: src/sketches/templates/template.rs
[tao]: https://github.com/tauri-apps/tao
[vite]: https://vite.dev/
[webview]: https://en.wikipedia.org/wiki/WebView
[wry]: https://github.com/tauri-apps/wry
