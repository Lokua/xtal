use std::cell::RefCell;
use std::collections::HashMap;

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
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn has(&self, name: &str, frame: Frame) -> bool {
        self.cache
            .borrow()
            .get(name)
            .map_or(false, |&(cached_frame, _)| cached_frame == frame)
    }

    pub fn store(&self, name: &str, frame: Frame, value: CachedValue) {
        self.cache
            .borrow_mut()
            .insert(name.to_string(), (frame, value));
    }

    pub fn get(&self, name: &str) -> Option<(Frame, CachedValue)> {
        self.cache.borrow().get(name).and_then(|x| Some(x.clone()))
    }

    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
}
