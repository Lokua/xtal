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
#[derive(Clone, Debug)]
pub struct LatticeContext {
    bpm: Bpm,
    window_rect: WindowRect,
}

impl LatticeContext {
    pub fn new(bpm: Bpm, window_rect: WindowRect) -> Self {
        Self { bpm, window_rect }
    }

    pub fn bpm(&self) -> Bpm {
        self.bpm.clone()
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

/// Secondary trait that all sketches must implement if they want to integrate
/// with the main runtime. Does not have to implemented manually. Use with:
/// ```rust
/// #[derive(SketchComponents)]
/// pub struct MySketch {}
/// ```
pub trait SketchDerived {
    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        None
    }
    fn controls_provided(&mut self) -> Option<&mut Controls> {
        self.controls().map(|provider| provider.controls_mut())
    }
    fn clear_color(&self) -> Rgba {
        Rgba::new(0.0, 0.0, 0.0, 1.0)
    }
}

pub trait SketchAll: Sketch + SketchDerived {}
impl<T: Sketch + SketchDerived> SketchAll for T {}
