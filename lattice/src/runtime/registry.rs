use indexmap::IndexMap;
use nannou::prelude::*;
use std::str;
use std::sync::{LazyLock, RwLock};

use crate::framework::prelude::*;

/// Register all of your project's sketches
///
/// # Example
/// ```rust,ignore
/// use lattice::prelude::*;
///
/// mod my_sketches;
/// use my_sketches::{a, b, c, d};
///
/// fn main() {
///     register!(a, b, c, d)
///     run();
/// }
/// ```
#[macro_export]
macro_rules! register {
    ($($module:ident),* $(,)?) => {
        {
            use $crate::REGISTRY;

            let mut registry = REGISTRY.write().unwrap();

            $(
                registry.register(
                    &$module::SKETCH_CONFIG,
                    |app, ctx| {
                        Box::new($module::init(
                            app,
                            ctx
                        )) as Box<dyn $crate::prelude::SketchAll>
                    }
                );
            )*

            registry.prepare();
            drop(registry);
        }
    };
}

type DynamicSketchFn = Box<
    dyn for<'a> Fn(&'a App, &Context) -> Box<dyn SketchAll + 'static>
        + Send
        + Sync,
>;

pub struct SketchInfo {
    pub config: &'static SketchConfig,
    pub factory: DynamicSketchFn,
}

pub static REGISTRY: LazyLock<RwLock<SketchRegistry>> =
    LazyLock::new(|| RwLock::new(SketchRegistry::new()));

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
        F: Fn(&App, &Context) -> Box<dyn SketchAll> + Send + Sync + 'static,
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
