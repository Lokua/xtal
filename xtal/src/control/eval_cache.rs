//! Cache for control values that are [parameter modulation sources][pmod].
//! These are evaluated per-frame before the consumers that depend on them so we
//! cache their results for the eventual subsequent request for their value. For
//! example:
//!
//! ```yaml
//! a:
//!   type: slider
//!
//! b:
//!   type: triangle
//!   beats: $slider
//! ```
//!
//! Here `a` is a source which must be evaluated before `b`. In any case,
//! whether `a` or `b` is requested first, there will 100% be a second request
//! for `a` from the UI, hence this cache.
//!
//! [pmod]: crate::control::param_mod
use std::cell::RefCell;

use crate::core::prelude::*;

type NodeName = String;
type Frame = u32;
type CachedValue = f32;

/// Per-frame cache keyed by control node name.
///
/// Values are only valid for the frame they were stored on. This allows a hot
/// parameter to be evaluated once while still serving later requests from UI or
/// dependent controls during the same frame.
#[derive(Debug, Default)]
pub struct EvalCache {
    cache: RefCell<HashMap<NodeName, (Frame, CachedValue)>>,
}

impl EvalCache {
    /// Returns whether `name` has a cached value for `frame`.
    pub fn has(&self, name: &str, frame: Frame) -> bool {
        if let Some(&(cached_frame, _)) = self.cache.borrow().get(name) {
            return cached_frame == frame;
        }
        false
    }

    /// Stores `value` for `name` on `frame`.
    pub fn store(&self, name: &str, frame: Frame, value: CachedValue) {
        self.cache
            .borrow_mut()
            .insert(name.to_string(), (frame, value));
    }

    /// Returns the cached value for `name` when it matches `frame`.
    pub fn get(&self, name: &str, frame: Frame) -> Option<CachedValue> {
        self.cache
            .borrow()
            .get(name)
            .and_then(|&(cached_frame, value)| {
                if cached_frame == frame {
                    Some(value)
                } else {
                    None
                }
            })
    }

    /// Clears all cached frame values.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
}
