use nannou::prelude::*;

/// A wrapper around nannou's `Rect` that is used to
/// provide "the" main window to sketches. Development on
/// nannou v0.19 is frozen and its app.main_window function
/// is unreliable (it returns the currently focused window,
/// not the main window). So intead of having to pass a window.id down
/// to every sketch we provide this rect as a convenience, since
/// having window dimensions and being able to check if a resize has
/// happened is a common need.
pub struct WindowRect {
    current: Rect,
    last: Rect,
}

impl WindowRect {
    pub fn new(initial: Rect) -> Self {
        Self {
            current: initial,
            last: initial,
        }
    }

    pub fn set_current(&mut self, rect: Rect) {
        self.current = rect;
    }

    /// Returns true if the window size has changed since the last time
    /// [`mark_unchanged`] was called. Use this in the `update` function
    /// when you want to perform an expensive operation only when needed.
    /// ```rust
    /// if m.wr.changed() {
    ///   // do stuff
    ///   //...
    ///   // But don't forget to mark it as changed!
    ///   m.wr.mark_unchanged()
    /// }
    /// ```
    /// Note that this will always return true the first time it is called
    /// or forever after that until mark_unchanged is called. This
    /// makes the code snippet above function as dual-purpose
    /// "init" style setup function which for convenience.
    pub fn changed(&self) -> bool {
        (self.current.w() != self.last.w())
            || (self.current.h() != self.last.h())
    }

    pub fn mark_unchanged(&mut self) {
        self.last = self.current;
    }

    pub fn w(&self) -> f32 {
        self.current.w()
    }
    pub fn h(&self) -> f32 {
        self.current.h()
    }

    /// "half width"
    pub fn hw(&self) -> f32 {
        self.current.w() / 2.0
    }
    /// "half height"
    pub fn hh(&self) -> f32 {
        self.current.h() / 2.0
    }

    /// "quarter width"
    pub fn qw(&self) -> f32 {
        self.current.w() / 4.0
    }
    /// "quarter height"
    pub fn qh(&self) -> f32 {
        self.current.h() / 4.0
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.w() / self.h()
    }

    pub fn resolution(&self) -> [f32; 2] {
        [self.w(), self.h()]
    }

    pub fn resolution_u32(&self) -> [u32; 2] {
        [self.w() as u32, self.h() as u32]
    }

    pub fn vec2(&self) -> Vec2 {
        vec2(self.w(), self.h())
    }

    // Methods delegated to self.current
    pub fn top(&self) -> f32 {
        self.current.top()
    }
    pub fn bottom(&self) -> f32 {
        self.current.bottom()
    }
    pub fn left(&self) -> f32 {
        self.current.left()
    }
    pub fn right(&self) -> f32 {
        self.current.right()
    }
    pub fn x(&self) -> f32 {
        self.current.x()
    }
    pub fn y(&self) -> f32 {
        self.current.y()
    }
    pub fn pad(&self, value: f32) -> Rect {
        self.current.pad(value)
    }
    pub fn pad_left(&self, value: f32) -> Rect {
        self.current.pad_left(value)
    }
    pub fn pad_right(&self, value: f32) -> Rect {
        self.current.pad_right(value)
    }
    pub fn pad_top(&self, value: f32) -> Rect {
        self.current.pad_top(value)
    }
    pub fn pad_bottom(&self, value: f32) -> Rect {
        self.current.pad_bottom(value)
    }
    pub fn top_left(&self) -> Point2 {
        self.current.top_left()
    }
    pub fn top_right(&self) -> Point2 {
        self.current.top_right()
    }
    pub fn bottom_left(&self) -> Point2 {
        self.current.bottom_left()
    }
    pub fn bottom_right(&self) -> Point2 {
        self.current.bottom_right()
    }
}
