//! Cache for control values that are [parameter modulation sources](pmod).
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
//! [pmod]: crate::framework::control::param_mod
use std::cell::RefCell;

use crate::framework::prelude::*;

type NodeName = String;
type Frame = u32;
type CachedValue = f32;

/// See [`crate:framework::control::eval_cache`]
#[derive(Debug, Default)]
pub struct EvalCache {
    cache: RefCell<HashMap<NodeName, (Frame, CachedValue)>>,
}

impl EvalCache {
    pub fn has(&self, name: &str, frame: Frame) -> bool {
        if let Some(&(cached_frame, _)) = self.cache.borrow().get(name) {
            return cached_frame == frame;
        }
        false
    }

    pub fn store(&self, name: &str, frame: Frame, value: CachedValue) {
        self.cache
            .borrow_mut()
            .insert(name.to_string(), (frame, value));
    }

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

    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
}
