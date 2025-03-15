use indexmap::IndexMap;
use nannou::prelude::*;
use once_cell::sync::Lazy;
use std::str;
use std::sync::RwLock;

use crate::framework::prelude::*;

#[macro_export]
macro_rules! register_sketches {
    ($registry:expr, $($module:ident),*) => {
        $(
            $registry.register(
                &$crate::sketches::$module::SKETCH_CONFIG,
                |app, ctx| {
                    Box::new($crate::sketches::$module::init(
                        app,
                        ctx
                    )) as Box<dyn SketchAll>
                }
            );
        )*
    };
}

type DynamicSketchFn = Box<
    dyn for<'a> Fn(&'a App, &LatticeContext) -> Box<dyn SketchAll + 'static>
        + Send
        + Sync,
>;

pub struct SketchInfo {
    pub config: &'static SketchConfig,
    pub factory: DynamicSketchFn,
}

pub static REGISTRY: Lazy<RwLock<SketchRegistry>> =
    Lazy::new(|| RwLock::new(SketchRegistry::new()));

pub struct SketchRegistry {
    sketches: IndexMap<String, SketchInfo>,
    names: Option<Vec<String>>,
}

impl SketchRegistry {
    fn new() -> Self {
        Self {
            sketches: IndexMap::new(),
            names: None,
        }
    }

    pub fn register<F>(&mut self, config: &'static SketchConfig, factory: F)
    where
        F: Fn(&App, &LatticeContext) -> Box<dyn SketchAll>
            + Send
            + Sync
            + 'static,
    {
        self.sketches.insert(
            config.name.to_string(),
            SketchInfo {
                config,
                factory: Box::new(factory),
            },
        );
        self.names = None;
    }

    pub fn get(&self, name: &str) -> Option<&SketchInfo> {
        self.sketches.get(name)
    }

    pub fn prepare(&mut self) {
        if self.names.is_none() {
            self.names = Some(self.sketches.keys().cloned().collect());
        }
    }

    pub fn names(&self) -> &Vec<String> {
        self.names.as_ref().expect(
            "Registry must be prepared before accessing names. \
                Call prepare() first.",
        )
    }
}
