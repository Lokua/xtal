> NOTE: this document for now will serve as a dumping ground until I figure out
> exactly how I want to organize more formal documentation

# Xtal & Nannou

Xtal is essentially one big Nannou app. The first major difference is that a
Xtal sketch must export a `SketchConfig` const containing metadata needed for
the runtime to properly boot a sketch. The second major difference is that
instead of the standalone `model`, `update`, and `view` functions as you find in
raw-Nannou, a Xtal sketch must provide an implementation of the `Sketch` trait.
You may also notice a 3rd context argument in each method not found in the
Nannou signatures – we'll get into that later – but besides these differences,
everything is the same as a Nannou app and Nannou is still the crate you're
likely to interact with the most in your code.

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

### Xtal Boilerplate

```rust
use xtal::prelude::*;
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

Now let's get into some of the benefits in the next section...

## ControlHub

At the heart of Xtal is the `ControlHub` struct (which we'll refer to as hub
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
controls. Hopefully this gives you a better idea of what Xtal provides on top of
Nannou.

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

This is just the tip of what the Animation module is capable of; for more
information consult the cargo docs.

## Control Scripting

While Xtal's various control and animation methods are easy to setup, it's a bit
of pain to have to restart the rust sketch every time you want to change an
animation or control configuration – especially as your sketch matures. For this
reason Xtal provides a script-like mechanism that uses yaml for configuration
and adds these controls dynamically and self-updates at runtime when the yaml
file is changed. You still have to take care to setup the routings in your
sketch (e.g. `let radius = self.hub.get("radius")`), but once these routings are
in place you are free to edit their ranges, values, timing, etc. It's also worth
knowing that Control Scripting makes certain things like disabling controls
based on the values of other controls and parameter modulation much easier than
they'd be in real code. Checkout any sketch in [xtal-sketches][xtal-sketches]
that has a corresponding yaml file of the same name for a working example or
[docs/control_script_reference.md](docs/control_script_reference.md) for
comprehensive documentation.

# User Interface

In the bottom of the UI is a console window that displays system alerts and
general operation feedback; at the top left of the console is a small (?) icon
you can press to enabled **Help Mode**, which will use the console to display
help information along with the keyboard shortcut for any control you hover
over.

# Audio

## Multichannel Audio

**Example**

```rust
let hub = ControlHubBuilder::new()
    .timing(Timing::new(ctx.bpm()))
    .audio(
        "bass_drum",
        AudioControlConfig {
            channel: 0,
            slew_limiter: SlewLimiter::default(),
            pre_emphasis: 0.0,
            detect: 0.0,
            range: (0.0, 1.0),
            value: 0.0,
        },
    )
    .audio(
        "snare_drum",
        AudioControlConfig {
            channel: 1,
            // You almost always want slew on Audio since it's so jumpy
            slew_limiter: SlewLimiter::new(0.65, 0.65),
            pre_emphasis: 0.0,
            detect: 0.0,
            range: (0.0, 1.0),
            value: 0.0,
        },
    )
    .build();

// later:
let bd = hub.get("bass_drum");
let sd = hub.get("snare_drum");
```

The `AudioControls` struct treats each audio channel as an individual control
signal with optional slew limiting, suitable for audio-rate or control-rate
signals. You can configure the audio device that is used in Xtal globally for
all sketches in the **UI > Settings** view. On my computer I'm using the [16
channel version of Blackhole][blackhole]. See below for how to set this up on
macOS.

### Aggregate Device Setup

![Mac Aggregate Device Setup](../assets/aggregate-device-multichannel.png)

> In the above setup I use 1-2 as the main outs and send the multichannel data
> out to channels 3-18 in my DAW which then appear on Blackhole channels 1-16

See [audio_controls_dev.rs](../src/sketches/dev/audio_controls_dev.rs) or
[cv_dev.rs](../src/sketches/dev/cv_dev.rs) for an example that uses CV.

## Single Channel, Multiband Audio (_experimental_)

See [audio_dev.rs](../src/sketches/dev/audio_dev.rs) for an example sketch.

The `Audio` struct in xtal is configured to process the first channel of
whatever audio device you have selected in the UI. I am currently doing this via
Aggregate Device on my Mac using [Blackhole 2ch][blackhole] to capture output
from my DAW (setup screenshots below). Note that this module is experimental and
doesn't integrate with the rest of Xtal as nicely as `AudioControls` does.

### Aggregate Device Setup

![Mac Aggregate Device Setup](../assets/aggregate-device-setup.png)

### Routing Audio to Blackhole 2ch `Out(3/4):In(1/2)`

> Note that Blackhole automatically routes whatever its output channels are to
> its own input, so sending audio out to Blackhole 3/4 will automatically appear
> on inputs 1/2 in this setup; you don't even need to configure the inputs in
> Ableton at all for this to work (just as long as you have the output config
> set to "Xtal" and enable the appropriate ouputs in the output config under
> Live's audio preferences)

![Ableton Live - Blackhole Track Routing](../assets/live-blackhole-track-routing.png)

# MIDI

**Example**

```rust
let hub = ControlHubBuilder::new()
    .timing(Timing::new(ctx.bpm()))
    // The incoming MIDI u8 values are always normalized to a 0..=1 range
    // name, (channel, controller), (min, max), default_value
    .midi("foo", (0, 0), (100.0, 500.0), 0.0)
    // midi_n = midi "normalized" - no min/max mapping beyond the default 0..=1
    .midi_n("bar", (0, 1), 0.0)
    .midi_n("baz", (0, 2), 0.0)
    .build();
```

MIDI clock, input, and output ports can be set in the UI > Settings view

### Loopback (Ableton)

To automate synth parameters in Ableton and Xtal parameters simultaneously from
_the same UI CC control in Live_ (as opposed to a physical control, in which
case you can skip this section), you need to enable MIDI loopback by sending
MIDI to `Xtal In` and also route `Xtal In` back in to Live to control
parameters. Here's the routing:

![Live MIDI Preferences](../assets/live-midi-prefs.png)

To use Ableton automation lanes to control Xtal params, follow these steps:

1. Create a MIDI track and clip and add CC automation to it.
2. In the tracks **MIDI To** router, select `IAC Driver Xtal In` and `Ch. 1`

Those steps are all you need to send MIDI to Xtal to control parameters. As for
controlling a live parameter with that same CC, follow these steps:

1. Play your clip containing the CC data
2. Stop the transport (this is important!)
3. Enter MIDI Mapping mode.
4. Locate the parameter to you want to map and select it (make sure it's the
   last thing you've clicked)
5. Press the Space bar to start the transport. This should do it!

See the [midi_test.rs sketch][midi-sketch] for an example of how to map a
control to something.

> Note: the above instructions are for working without a MIDI controller. When
> working with a MIDI controller you can just map the MIDI control to an Ableton
> device knob that can send CC out to Xtal and also map the controller to an
> Ableton parameter. In this case _you do not_ want Xtal enabled in Ableton's
> MIDI Input ports at all as that just complicates things.

### Sync Recordings

With MIDI ports configured in your DAW to send clock to Xtal, Xtal is already in
a place where you can perfectly sync video recordings with audio from your DAW.
Below are steps to setup Ableton Live such that you can record audio and video
simultaneously when you press Play in the DAW (if you only want to record video
you can just do steps 2 and 4):

1. In Ableton > Preferences > Record, make sure **Start Transport With Record**
   is set to **Off**
2. Hit **Q Rec** in Xtal.
3. Arm tracks in Ableton, arm the transport (Record button)
4. Now, pressing play in Ableton will also initiate recording in Xtal, likewise
   pressing Stop in Ableton will stop recording in Xtal.

# Open Sound Control (OSC)

While MIDI is great for controlling parameters in the case that a MIDI
controller can send 14bit high resolution MIDI, it sucks otherwise (128 values
just isn't enough precision for smooth parameter automation). For this reason
Xtal supports OSC via [Nannou OSC][nannou-osc] and comes with two MaxForLive
devices designed to make integration with Ableton Live simpler.

**Example**

```rust
let hub = ControlHubBuilder::new()
    .timing(Timing::new(ctx.bpm()))
    // address (without leading slash), (min, max), default_value
    .osc("foo", (100.0, 500.0), 22.0)
    // Same as above without range mapping (assumes incoming 0.0..=1.0 range)
    .osc_n("bar", 22.0)
    .osc_n("baz/qux", 22.0)
    .build();
```

### L.OscTransport

[assets/L.OscTransport.amxd][osc-transport]

![L.OscTransport MaxForLive Device](../assets/osc-transport.png)

Place this on any track in Ableton and it will send high precision clock and
exact transport location to Xtal. This should be preferred over using MIDI
Timing however you should still make sure MIDI ports between Ableton and Xtal
are configured properly as Xtal still depends on MIDI clock for starting,
stopping, and syncing video recordings. The default host and port align with
what Xtal expects and can be left alone, though you can configure this in
[src/config.rs][config].

### L.OscSend

[assets/L.OscSend.amxd][osc-send]

![L.OscSend MaxForLive Device](../assets/osc-send.png)

A super basic OSC value sender. While there are much fancier MaxForLive devices
that can send OSC, the "official" OSC Send device that comes with Ableton's
Connection Kit does _not_ send high resolution data, which defeats the entire
purpose!orLive devices designed to make integration with Ableton Live simpler.

# Tips

## Change Detection

For sketches where every drop of performance matters, there are some
optimizations you can use.

### Window Resizing

If you are setting up grids or using positioning that is dependent on the
current size of the window, you can use `Context::window_rect` to only update
model data on resize:

```rust
fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
    let wr = ctx.window_rect();

    if wr.changed() {
        self.model.expensive_setup(wr.w(), wr.h());
        wr().mark_unchanged(); // <- don't forget this
    }
```

Note that `wr.changed()` will _always_ return true on first render and for that
reason you should defer expensive initializations until this point in your code
instead of the `init` function.

### Control Changes

Similar to only recalculating certain data when the window changes, the
`ControlHub` also provides change detection for `UIControls`:

```rust
fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
    if self.hub.changed() {
        self.model.do_stuff();
        self.hub.mark_unchanged(); // <- don't forget this
    }
```

It's unlikely that you'll want to reformat data on your model _every_ time _any_
control changes, but more likely when a specific control or set of controls
changes:

```rust
fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
    if self.hub.any_changed_in(&["algorithm", "object_count"]) {
        self.model.do_stuff();
        self.hub.mark_unchanged(); // <- don't forget this
    }
```

And again, just like `WindowRect::changed`, this _always_ returns true on the
first render, so as a general rule/pattern - use empty data structures in `init`
and then update them in one of these changed blocks if you need to support
complex runtime data realignments.

```rust
impl Sketch for MySketch {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        if wr.changed() || self.hub.any_changed_in(&["algorithm"]) {
            self.model.do_stuff();
            wr.mark_unchanged();
            self.hub.mark_unchanged();
        }
    }
```

## Clearing

The Clear button in the UI serves as a mechanism to let sketches know when they
can "reset" a sketch or clear any trails caused by low background alpha values.

### Example: Resetting Data

```rust
fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
    if ctx.should_clear() {
        self.drops.clear();
    }
}
```

### Example: Clearing "Trails"

Use the `Context::background` method to simultaneously setup a clear color and a
background color. The clear color will be the same as the background with alpha
set to 1.0.

```rust
fn view(&self, app: &App, frame: Frame, ctx: &Context) {
    let draw = app.draw();
    ctx.background(&frame, &draw, hsla(0.0, 0.0, 0.3, 0.02));
```

# General Resources

- https://sotrh.github.io/learn-wgpu
- https://inconvergent.net/generative/
- http://www.complexification.net/
- https://n-e-r-v-o-u-s.com/projects/albums/floraform-system/
- https://www.andylomas.com/cellularFormImages.html
- http://www.complexification.net/gallery/machines/sandstroke/
- https://thebookofshaders.com/
- https://github.com/jasonwebb/2d-space-colonization-experiments
- https://paulbourke.net/geometry/
- https://easings.net/

[blackhole]: https://existential.audio/blackhole/
[coding-train]: https://thecodingtrain.com/
[config]: ../src/config.rs
[control-script-test]: src/sketches/scratch/control_script_test.rs
[ffmpeg]: https://ffmpeg.org/
[insta]: https://www.instagram.com/lokua/
[just]: https://github.com/casey/just
[xtal-sketches]: ../xtal-sketches/sketches
[midi-sketch]: src/sketches/midi_test.rs
[nannou]: https://github.com/nannou-org/nannou
[nannou-osc]: https://github.com/nannou-org/nannou/tree/master/nannou_osc
[osc-send]: ../assets/L.OscSend.amxd
[osc-transport]: ../assets/L.OscTransport.amxd
[p5]: https://github.com/Lokua/p5/tree/main
[template]: src/sketches/templates/template.rs
[tao]: https://github.com/tauri-apps/tao
[vite]: https://vite.dev/
[webview]: https://en.wikipedia.org/wiki/WebView
[wry]: https://github.com/tauri-apps/wry
