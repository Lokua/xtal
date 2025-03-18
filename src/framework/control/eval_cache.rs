use std::cell::RefCell;

use crate::framework::prelude::*;

type NodeName = String;
type Frame = u32;
type CachedValue = f32;

#[derive(Debug)]
pub struct EvalCache {
    cache: RefCell<HashMap<NodeName, (Frame, CachedValue)>>,
}

impl EvalCache {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::default()),
        }
    }

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
