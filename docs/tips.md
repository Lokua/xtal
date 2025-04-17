> NOTE: this document for now will serve as a dumping ground until I figure out
> exactly how I want to organize more formal documentation

# User Interface

TODO

## Keyboard Shortcuts

In the bottom of the UI is a console window that displays system alerts and
general operation feedback; in the top left is a small (?) icon you can press to
enabled **Help Mode**, which will use the console to display help information
for any control you hover over.

| Feature         | Keyboard Shortcut |
| --------------- | ----------------- |
| Play/Pause      | P                 |
| Advance         | A                 |
| Reset           | R                 |
| Clear           | -                 |
| Capture Image   | I (i key)         |
| Queue           | -                 |
| Record          | -                 |
| Save            | Cmd S or Shift S  |
| Settings        | , (comma)         |
| Reset Sketch    | Shift Cmd S       |
| Perf Mode       | -                 |
| Tap             | Space             |
| Exclusions      | E                 |
| Randomize       | Cmd R             |
| Snapshots       | S                 |
| Save Snap       | Shift Digit       |
| Recall Snap     | Cmd Digit         |
| Transition Time | -                 |
| Fullscreen      | F                 |
| Focus Main      | M                 |
| Focus GUI       | G                 |

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
signals. You can configure the audio device that used in Lattice globally for
all sketches in the UI > Settings view. On my computer I'm using the [16 channel
version of Blackhole][blackhole]. See [docs/tips.md](docs/tips.md#Audio) for
more details on that.

### Aggregate Device Setup

![Mac Aggregate Device Setup](assets/aggregate-device-multichannel.png)

> In the above setup I use 1-2 as the main outs and send the multichannel data
> out to channels 3-18 in my DAW which then appear on Blackhole channels 1-16

See [audio_controls_dev.rs](src/sketches/dev/audio_controls_dev.rs) or
[cv_dev.rs](src/sketches/dev/cv_dev.rs) for an example that uses CV.

## Single Channel, Multiband Audio (_experimental_)

See [audio_dev.rs](src/sketches/dev/audio_dev.rs) for an example sketch.

The `Audio` struct in lattice is configured to process the first channel of
whatever audio device you have selected in the UI. I am currently doing this via
Aggregate Device on my Mac using [Blackhole 2ch][blackhole] to capture output
from my DAW (setup screenshots below). Note that this module is experimental and
doesn't integrate with the rest of Lattice as nicely as `AudioControls` does.

### Aggregate Device Setup

![Mac Aggregate Device Setup](assets/aggregate-device-setup.png)

### Routing Audio to Blackhole 2ch `Out(3/4):In(1/2)`

> Note that Blackhole automatically routes whatever its output channels are to
> its own input, so sending audio out to Blackhole 3/4 will automatically appear
> on inputs 1/2 in this setup; you don't even need to configure the inputs in
> Ableton at all for this to work (just as long as you have the output config
> set to "Lattice" and enable the appropriate ouputs in the output config under
> Live's audio preferences)

![Ableton Live - Blackhole Track Routing](assets/live-blackhole-track-routing.png)

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

To automate synth parameters in Ableton and Lattice parameters simultaneously
from _the same UI CC control in Live_ (as opposed to a physical control, in
which case you can skip this section), you need to enable MIDI loopback by
sending MIDI to `Lattice In` and also route `Lattice In` back in to Live to
control parameters. Here's the routing:

![Live MIDI Preferences](assets/live-midi-prefs.png)

To use Ableton automation lanes to control Lattice params, follow these steps:

1. Create a MIDI track and clip and add CC automation to it.
2. In the tracks **MIDI To** router, select `IAC Driver Lattice In` and `Ch. 1`

Those steps are all you need to send MIDI to Lattice to control parameters. As
for controlling a live parameter with that same CC, follow these steps:

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
> device knob that can send CC out to Lattice and also map the controller to an
> Ableton parameter. In this case _you do not_ want Lattice enabled in Ableton's
> MIDI Input ports at all as that just complicates things.

### Sync Recordings

With MIDI ports configured in your DAW to send clock to Lattice, Lattice is
already in a place where you can perfectly sync video recordings with audio from
your DAW. Below are steps to setup Ableton Live such that you can record audio
and video simultaneously when you press Play in the DAW (if you only want to
record video you can just do steps 2 and 4):

1. In Ableton > Preferences > Record, make sure **Start Transport With Record**
   is set to **Off**
2. Hit **Q Rec** in Lattice.
3. Arm tracks in Ableton, arm the transport (Record button)
4. Now, pressing play in Ableton will also initiate recording in Lattice,
   likewise pressing Stop in Ableton will stop recording in Lattice.

# Open Sound Control (OSC)

While MIDI is great for controlling parameters in the case that a MIDI
controller can send 14bit high resolution MIDI, it sucks otherwise (128 values
just isn't enough precision for smooth parameter automation). For this reason
Lattice supports OSC via [Nannou OSC][nannou-osc] and comes with two MaxForLive
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

![L.OscTransport MaxForLive Device](assets/osc-transport.png)

Place this on any track in Ableton and it will send high precision clock and
exact transport location to Lattice. This should be preferred over using MIDI
Timing however you should still make sure MIDI ports between Ableton and Lattice
are configured properly as Lattice still depends on MIDI clock for starting,
stopping, and syncing video recordings. The default host and port align with
what Lattice expects and can be left alone, though you can configure this in
[src/config.rs][config].

### L.OscSend

[assets/L.OscSend.amxd][osc-send]

![L.OscSend MaxForLive Device](assets/osc-send.png)

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

## General Resources

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
[nannou-osc]: https://github.com/nannou-org/nannou/tree/master/nannou_osc
[osc-send]: assets/L.OscSend.amxd
[osc-transport]: assets/L.OscTransport.amxd
[p5]: https://github.com/Lokua/p5/tree/main
[template]: src/sketches/templates/template.rs
[tao]: https://github.com/tauri-apps/tao
[vite]: https://vite.dev/
[webview]: https://en.wikipedia.org/wiki/WebView
[wry]: https://github.com/tauri-apps/wry
