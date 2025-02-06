use nannou::prelude::*;

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

    pub fn hw(&self) -> f32 {
        self.current.w() / 2.0
    }
    pub fn hh(&self) -> f32 {
        self.current.h() / 2.0
    }

    pub fn qw(&self) -> f32 {
        self.current.w() / 4.0
    }
    pub fn qh(&self) -> f32 {
        self.current.h() / 4.0
    }

    pub fn w_(&self, division: f32) -> f32 {
        self.current.w() / division
    }
    pub fn h_(&self, division: f32) -> f32 {
        self.current.w() / division
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
