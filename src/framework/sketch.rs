use nannou::prelude::*;

use super::prelude::*;

pub struct SketchConfig {
    pub name: &'static str,
    pub display_name: &'static str,
    pub fps: f32,
    pub bpm: f32,
    pub w: i32,
    pub h: i32,
    pub gui_w: Option<u32>,
    pub gui_h: Option<u32>,
    pub play_mode: PlayMode,
}

#[derive(PartialEq)]
pub enum PlayMode {
    // normal
    Loop,
    // Sketch starts in paused state,
    // auto advanced when controls are changed
    Advance,
    // Same as advance, but only advances if Adv. is pressed
    ManualAdvance,
}

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
}
