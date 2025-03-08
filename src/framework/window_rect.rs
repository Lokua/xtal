use nannou::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct WindowRectState {
    current: Rect,
    last: Rect,
}

/// A wrapper around nannou's `Rect` that is used to provide "the" main window
/// to sketches. Development on nannou v0.19 is frozen and its app.main_window
/// function is unreliable (it returns the currently focused window, not the
/// main window). So instead of having to pass a window.id down to every sketch
/// we provide this rect as a convenience, since having window dimensions and
/// being able to check if a resize has happened is a common need.
#[derive(Clone, Debug)]
pub struct WindowRect {
    state: Rc<RefCell<WindowRectState>>,
}

impl WindowRect {
    pub fn new(initial: Rect) -> Self {
        Self {
            state: Rc::new(RefCell::new(WindowRectState {
                current: initial,
                last: initial,
            })),
        }
    }

    pub fn set_current(&mut self, rect: Rect) {
        self.state.borrow_mut().current = rect;
    }

    /// Returns true if the window size has changed since the last time
    /// [`Self::mark_unchanged`] was called. Use this in the `update` function
    /// when you want to perform an expensive operation only when needed.
    /// ```rust
    /// let wr = ctx.window_rect();
    ///
    /// if wr.changed() {
    ///   // do stuff
    ///   //...
    ///   // But don't forget to mark it as unchanged or
    ///   // this block will run every frame!
    ///   wr.mark_unchanged()
    /// }
    /// ```
    /// Note that this will always return true the first time it is called or
    /// forever after that until mark_unchanged is called. This makes the code
    /// snippet above function as dual-purpose "init" style setup function which
    /// is pretty convenient.
    pub fn changed(&self) -> bool {
        let state = self.state.borrow();
        (state.current.w() != state.last.w())
            || (state.current.h() != state.last.h())
    }

    pub fn mark_unchanged(&mut self) {
        let mut state = self.state.borrow_mut();
        state.last = state.current;
    }

    pub fn w(&self) -> f32 {
        self.state.borrow().current.w()
    }

    pub fn h(&self) -> f32 {
        self.state.borrow().current.h()
    }

    /// "half width"
    pub fn hw(&self) -> f32 {
        self.state.borrow().current.w() / 2.0
    }

    /// "half height"
    pub fn hh(&self) -> f32 {
        self.state.borrow().current.h() / 2.0
    }

    /// "quarter width"
    pub fn qw(&self) -> f32 {
        self.state.borrow().current.w() / 4.0
    }

    /// "quarter height"
    pub fn qh(&self) -> f32 {
        self.state.borrow().current.h() / 4.0
    }

    pub fn wh(&self) -> (f32, f32) {
        (self.w(), self.h())
    }

    pub fn aspect_ratio(&self) -> f32 {
        let state = self.state.borrow();
        state.current.w() / state.current.h()
    }

    pub fn resolution(&self) -> [f32; 2] {
        let state = self.state.borrow();
        [state.current.w(), state.current.h()]
    }

    pub fn resolution_u32(&self) -> [u32; 2] {
        let state = self.state.borrow();
        [state.current.w() as u32, state.current.h() as u32]
    }

    pub fn vec2(&self) -> Vec2 {
        let state = self.state.borrow();
        vec2(state.current.w(), state.current.h())
    }

    pub fn rect(&self) -> Rect {
        self.state.borrow().current
    }

    // Methods delegated to self.current

    pub fn top(&self) -> f32 {
        self.state.borrow().current.top()
    }

    pub fn bottom(&self) -> f32 {
        self.state.borrow().current.bottom()
    }

    pub fn left(&self) -> f32 {
        self.state.borrow().current.left()
    }

    pub fn right(&self) -> f32 {
        self.state.borrow().current.right()
    }

    pub fn x(&self) -> f32 {
        self.state.borrow().current.x()
    }

    pub fn y(&self) -> f32 {
        self.state.borrow().current.y()
    }

    pub fn pad(&self, value: f32) -> Rect {
        self.state.borrow().current.pad(value)
    }

    pub fn pad_left(&self, value: f32) -> Rect {
        self.state.borrow().current.pad_left(value)
    }

    pub fn pad_right(&self, value: f32) -> Rect {
        self.state.borrow().current.pad_right(value)
    }

    pub fn pad_top(&self, value: f32) -> Rect {
        self.state.borrow().current.pad_top(value)
    }

    pub fn pad_bottom(&self, value: f32) -> Rect {
        self.state.borrow().current.pad_bottom(value)
    }

    pub fn top_left(&self) -> Point2 {
        self.state.borrow().current.top_left()
    }

    pub fn top_right(&self) -> Point2 {
        self.state.borrow().current.top_right()
    }

    pub fn bottom_left(&self) -> Point2 {
        self.state.borrow().current.bottom_left()
    }

    pub fn bottom_right(&self) -> Point2 {
        self.state.borrow().current.bottom_right()
    }
}
