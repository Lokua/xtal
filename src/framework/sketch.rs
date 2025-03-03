use nannou::prelude::*;

use super::control_provider::ControlProvider;
use super::prelude::*;

/// A configuration that all sketches must export in order to integrate
/// with the main Lattice runtime.
pub struct SketchConfig {
    /// Must be unique among all sketches
    pub name: &'static str,

    /// The name that will show up in the title bar of the window
    pub display_name: &'static str,

    /// The frame rate that will be provided to the global frame-count provider
    /// to keep everything including animations in sync
    pub fps: f32,

    /// The musical tempo at which animations will sync to
    pub bpm: f32,

    /// The default width the window should open at
    pub w: i32,

    /// The default height the window should open at
    pub h: i32,

    /// Lattice provides a sensible default for control window's width,
    /// but you can override this in the case you have really long
    /// parameter names
    pub gui_w: Option<u32>,

    /// The height of the control window. I've been unable to derive this
    /// from the number of controls - there is some weird quirk in the version
    /// of egui that ships with nannou, so until we get around that this must be
    /// provided and increased manually as your count of controls grows
    pub gui_h: Option<u32>,

    /// See [`PlayMode`]
    pub play_mode: PlayMode,
}

#[derive(PartialEq)]
pub enum PlayMode {
    // normal
    Loop,

    // Sketch starts in paused state then auto advanced when controls are changed
    Advance,

    // Same as advance, but only advances if the `Adv.` button
    // in the GUI is pressed
    ManualAdvance,
}

/// Context passed down from the Lattice runtime. This is similar to how
/// `nannou` provides an `app`, `ctx` will provide useful data for sketches.
#[derive(Debug)]
pub struct LatticeContext {
    pub bpm: Bpm,
    pub window_rect: WindowRect,
}

impl LatticeContext {
    pub fn new(bpm: Bpm, window_rect: WindowRect) -> Self {
        Self { bpm, window_rect }
    }

    pub fn window_rect(&self) -> WindowRect {
        self.window_rect.clone()
    }
}

/// Core trait for type erasure - all sketches must implement this
pub trait Sketch {
    fn update(&mut self, app: &App, update: Update, ctx: &LatticeContext);
    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext);
    fn event(&mut self, _app: &App, _event: &Event) {}
}

pub trait SketchDerived {
    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        None
    }
    fn controls_provided(&mut self) -> Option<&mut Controls> {
        self.controls().map(|provider| provider.as_controls_mut())
    }
    fn clear_color(&self) -> Rgba {
        Rgba::new(0.0, 0.0, 0.0, 1.0)
    }
    fn window_rect(&mut self) -> Option<&mut WindowRect> {
        None
    }
    fn set_window_rect(&mut self, rect: Rect) {
        if let Some(window_rect) = self.window_rect() {
            window_rect.set_current(rect);
        }
    }
}

pub trait SketchAll: Sketch + SketchDerived {}
impl<T: Sketch + SketchDerived> SketchAll for T {}

#[deprecated(note = "Use Sketch trait directly instead")]
pub trait SketchModel {
    fn controls(&mut self) -> Option<&mut impl ControlProvider> {
        None::<&mut Controls>
    }

    fn clear_color(&self) -> Rgba {
        Rgba::new(0.0, 0.0, 0.0, 0.0)
    }

    fn window_rect(&mut self) -> Option<&mut WindowRect> {
        None
    }

    fn set_window_rect(&mut self, rect: Rect) {
        if let Some(window_rect) = self.window_rect() {
            window_rect.set_current(rect);
        }
    }

    fn event(&mut self, _app: &App, _event: &Event) {}
}

/// Adapter to instantiate Sketch for legacy SketchModel types
pub struct SketchAdapter<S> {
    model: S,
    update_fn: fn(&App, &mut S, Update),
    view_fn: fn(&App, &S, Frame),
    controls_fn: Option<fn(&mut S) -> Option<&mut dyn ControlProvider>>,
    clear_color_fn: Option<fn(&S) -> Rgba>,
    window_rect_fn: Option<fn(&mut S) -> Option<&mut WindowRect>>,
    set_window_rect_fn: Option<fn(&mut S, Rect)>,
}

impl<S> SketchAdapter<S> {
    pub fn new(
        model: S,
        update_fn: fn(&App, &mut S, Update),
        view_fn: fn(&App, &S, Frame),
        controls_fn: Option<fn(&mut S) -> Option<&mut dyn ControlProvider>>,
        clear_color_fn: Option<fn(&S) -> Rgba>,
        window_rect_fn: Option<fn(&mut S) -> Option<&mut WindowRect>>,
        set_window_rect_fn: Option<fn(&mut S, Rect)>,
    ) -> Self {
        Self {
            model,
            update_fn,
            view_fn,
            controls_fn,
            clear_color_fn,
            window_rect_fn,
            set_window_rect_fn,
        }
    }
}

impl<S> Sketch for SketchAdapter<S> {
    fn update(&mut self, app: &App, update: Update, _ctx: &LatticeContext) {
        (self.update_fn)(app, &mut self.model, update);
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &LatticeContext) {
        (self.view_fn)(app, &self.model, frame);
    }

    fn event(&mut self, _app: &App, _event: &Event) {}
}

impl<S> SketchDerived for SketchAdapter<S> {
    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        if let Some(f) = self.controls_fn {
            f(&mut self.model).map(|c| c as &mut dyn ControlProvider)
        } else {
            None
        }
    }

    fn clear_color(&self) -> Rgba {
        if let Some(f) = self.clear_color_fn {
            f(&self.model)
        } else {
            Rgba::new(0.0, 0.0, 0.0, 1.0)
        }
    }

    fn window_rect(&mut self) -> Option<&mut WindowRect> {
        if let Some(f) = self.window_rect_fn {
            f(&mut self.model)
        } else {
            None
        }
    }

    fn set_window_rect(&mut self, rect: Rect) {
        if let Some(f) = self.set_window_rect_fn {
            f(&mut self.model, rect);
        }
    }
}
