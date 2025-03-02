use nannou::prelude::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::str;
use std::sync::RwLock;

use crate::framework::prelude::*;

#[macro_export]
macro_rules! register_legacy_sketches {
    ($registry:expr, $($module:ident),*) => {
        $(
            $registry.register(
                &crate::sketches::$module::SKETCH_CONFIG,
                |app, ctx| {
                    let model = crate::sketches::$module::init_model(
                        app,
                        WindowRect::new(ctx.window_rect.rect())
                    );
                    Box::new(SketchAdapter::new(
                        model,
                        crate::sketches::$module::update,
                        crate::sketches::$module::view,
                        Some(|model: &mut _| model.controls()),
                        Some(|model: &_| model.clear_color()),
                        Some(|model: &mut _| model.window_rect()),
                        Some(|model: &mut _, rect| model.set_window_rect(rect)),
                    )) as Box<dyn SketchAll>
                }
            );
        )*
    };
}

#[macro_export]
macro_rules! register_sketches {
    ($registry:expr, $($module:ident),*) => {
        $(
            $registry.register(
                &crate::sketches::$module::SKETCH_CONFIG,
                |app, ctx| {
                    Box::new(crate::sketches::$module::init(
                        app,
                        ctx
                    )) as Box<dyn SketchAll>
                }
            );
        )*
    };
}

pub struct SketchInfo {
    pub config: &'static SketchConfig,
    pub factory: Box<
        dyn for<'a> Fn(&'a App, LatticeContext) -> Box<dyn SketchAll + 'static>
            + Send
            + Sync,
    >,
}

pub static REGISTRY: Lazy<RwLock<SketchRegistry>> =
    Lazy::new(|| RwLock::new(SketchRegistry::new()));

pub struct SketchRegistry {
    sketches: HashMap<String, SketchInfo>,
    sorted_names: Option<Vec<String>>,
}

impl SketchRegistry {
    fn new() -> Self {
        Self {
            sketches: HashMap::new(),
            sorted_names: None,
        }
    }

    pub fn register<F>(&mut self, config: &'static SketchConfig, factory: F)
    where
        F: Fn(&App, LatticeContext) -> Box<dyn SketchAll>
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
        self.sorted_names = None;
    }

    pub fn get(&self, name: &str) -> Option<&SketchInfo> {
        self.sketches.get(name)
    }

    pub fn prepare(&mut self) {
        if self.sorted_names.is_none() {
            let mut names: Vec<String> =
                self.sketches.keys().cloned().collect();
            names.sort();
            self.sorted_names = Some(names);
        }
    }

    pub fn names(&self) -> &Vec<String> {
        self.sorted_names.as_ref().expect(
            "Registry must be prepared before accessing names. \
                Call prepare() first.",
        )
    }
}
