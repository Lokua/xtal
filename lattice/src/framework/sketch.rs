use nannou::color::IntoLinSrgba;
use nannou::draw::properties::ColorScalar;
use nannou::prelude::*;

use super::prelude::*;
use crate::runtime::app::ClearFlag;

/// A configuration that all sketches must export in order to integrate
/// with the main Lattice runtime.
pub struct SketchConfig {
    /// Must be unique among all sketches
    pub name: &'static str,

    /// The name that will be displayed in the window titles and sketch selector
    pub display_name: &'static str,

    /// See [`PlayMode`]
    pub play_mode: PlayMode,

    /// The frame rate that will be provided to the global frame-count provider
    /// to keep everything including animations in sync
    pub fps: f32,

    /// The musical tempo at which animations will sync to
    pub bpm: f32,

    /// The default width the main window should open at
    pub w: i32,

    /// The default height the main window should open at
    pub h: i32,
}

#[derive(PartialEq)]
pub enum PlayMode {
    /// Continuously run a sketch at the sketch's provided frame rate
    Loop,

    /// Sketch starts in paused state then auto advanced when controls are changed
    Advance,

    /// Same as advance, but only advances if the `Advance` button or `A` key is
    /// pressed
    ManualAdvance,
}

/// Context passed down from the Lattice runtime
#[derive(Clone, Debug)]
pub struct Context {
    bpm: Bpm,
    clear_flag: ClearFlag,
    window_rect: WindowRect,
}

impl Context {
    pub fn new(
        bpm: Bpm,
        clear_flag: ClearFlag,
        window_rect: WindowRect,
    ) -> Self {
        Self {
            bpm,
            clear_flag,
            window_rect,
        }
    }

    /// The global living BPM value used by all timing systems
    pub fn bpm(&self) -> Bpm {
        self.bpm.clone()
    }

    /// Accessor for the main window's `Rect` instance, wrapped in our own
    /// [`WindowRect`] which provides a change detection mechanism and other
    /// useful helpers
    pub fn window_rect(&self) -> WindowRect {
        self.window_rect.clone()
    }

    /// True for a single frame after pressing **Clear**
    pub fn should_clear(&self) -> bool {
        self.clear_flag.get()
    }

    /// A background color helper with support for clearing the Nannou
    /// [`nannou::frame::Frame`] via the **Clear** button in the UI as well as
    /// previous frame "trails" when background alpha is low
    pub fn background<C>(&self, frame: &Frame, draw: &Draw, color: C)
    where
        C: IntoLinSrgba<ColorScalar> + Clone,
    {
        if self.should_clear() {
            let (r, g, b, _) = color.clone().into_lin_srgba().into_components();
            let color = LinSrgba::new(r, g, b, 1.0);
            frame.clear(color);
        }

        let wr = self.window_rect();
        draw.rect().x_y(0.0, 0.0).w_h(wr.w(), wr.h()).color(color);
    }
}

/// Core trait for type erasure — all sketches must implement this
pub trait Sketch {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}
    fn event(&mut self, _app: &App, _event: &Event) {}
    fn view(&self, app: &App, frame: Frame, ctx: &Context);
}

/// Secondary trait that all sketches must implement in order to integrate with
/// the Lattice runtime. Does not have to be implemented manually. Use with:
/// ```rust
/// #[derive(SketchComponents)]
/// pub struct MySketch {}
/// ```
pub trait SketchDerived {
    fn hub(&mut self) -> Option<&mut dyn ControlHubProvider>;
}

#[doc(hidden)]
/// Trait used to enable dynamically loading sketches at runtime via
/// [`crate::REGISTRY`]
pub trait SketchAll: Sketch + SketchDerived {}
impl<T: Sketch + SketchDerived> SketchAll for T {}
