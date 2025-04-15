# Lattice

> NOTE: this document for now will serve as a dumping ground until I figure out
> exactly how I want to organize more formal documentation

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
