use nannou::prelude::*;

use super::control_provider::ControlProvider;
use super::prelude::*;

/// A configuration that all sketches must export in order to integrate
/// with the main Lattice runtime.
pub struct SketchConfig {
    /// Not used but may be used in the future
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

/// Core trait for type erasure - all sketches must implement this
pub trait Sketch {
    fn update(&mut self, app: &App, update: Update);
    fn view(&self, app: &App, frame: Frame);

    fn event(&mut self, _app: &App, _event: &Event) {}
    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        None
    }
    fn controls_provided(&mut self) -> Option<&mut Controls> {
        self.controls().map(|provider| provider.as_controls_mut())
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
}

impl<T: SketchModel> Sketch for T {
    fn update(&mut self, _app: &App, _update: Update) {
        panic!("update() not implemented for this SketchModel")
    }

    fn view(&self, _app: &App, _frame: Frame) {
        panic!("view() not implemented for this SketchModel")
    }

    fn event(&mut self, app: &App, event: &Event) {
        SketchModel::event(self, app, event)
    }

    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        SketchModel::controls(self).map(|c| c as &mut dyn ControlProvider)
    }

    fn clear_color(&self) -> nannou::color::Rgba {
        SketchModel::clear_color(self)
    }

    fn set_window_rect(&mut self, rect: Rect) {
        SketchModel::set_window_rect(self, rect);
    }
}

/// Adapter to implement Sketch for legacy SketchModel types
pub struct SketchAdapter<S: Sketch> {
    model: S,
    update_fn: fn(&App, &mut S, Update),
    view_fn: fn(&App, &S, Frame),
}

impl<S: Sketch> SketchAdapter<S> {
    pub fn new(
        model: S,
        update_fn: fn(&App, &mut S, Update),
        view_fn: fn(&App, &S, Frame),
    ) -> Self {
        Self {
            model,
            update_fn,
            view_fn,
        }
    }
}

impl<S: Sketch> Sketch for SketchAdapter<S> {
    fn update(&mut self, app: &App, update: Update) {
        (self.update_fn)(app, &mut self.model, update);
    }

    fn view(&self, app: &App, frame: Frame) {
        (self.view_fn)(app, &self.model, frame);
    }

    fn event(&mut self, app: &App, event: &Event) {
        self.model.event(app, event);
    }

    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        self.model.controls()
    }

    fn clear_color(&self) -> nannou::color::Rgba {
        self.model.clear_color()
    }

    fn set_window_rect(&mut self, rect: Rect) {
        self.model.set_window_rect(rect);
    }
}
