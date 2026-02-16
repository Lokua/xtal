use std::collections::HashMap;

use crate::sketch::{Sketch, SketchConfig};

type SketchFactory = Box<dyn Fn() -> Box<dyn Sketch> + Send + Sync + 'static>;

pub struct SketchEntry {
    pub config: &'static SketchConfig,
    pub factory: SketchFactory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SketchCategory {
    pub title: String,
    pub enabled: bool,
    pub sketches: Vec<String>,
}

#[derive(Default)]
pub struct RuntimeRegistry {
    entries: HashMap<String, SketchEntry>,
    ordered_names: Vec<String>,
    categories: Vec<SketchCategory>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(
        &mut self,
        config: &'static SketchConfig,
        factory: F,
    ) -> Result<(), String>
    where
        F: Fn() -> Box<dyn Sketch> + Send + Sync + 'static,
    {
        let name = config.name.to_string();
        if self.entries.contains_key(&name) {
            return Err(format!("duplicate sketch registration: {}", name));
        }

        self.ordered_names.push(name.clone());
        self.entries.insert(
            name,
            SketchEntry {
                config,
                factory: Box::new(factory),
            },
        );

        Ok(())
    }

    pub fn define_category(
        &mut self,
        title: impl Into<String>,
        enabled: bool,
        sketches: Vec<String>,
    ) -> Result<(), String> {
        let title = title.into();

        for name in &sketches {
            if !self.entries.contains_key(name) {
                return Err(format!(
                    "category '{}' references unknown sketch '{}'",
                    title, name
                ));
            }
        }

        self.categories.push(SketchCategory {
            title,
            enabled,
            sketches,
        });

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&SketchEntry> {
        self.entries.get(name)
    }

    pub fn sketch_names(&self) -> &[String] {
        &self.ordered_names
    }

    pub fn first_sketch_name(&self) -> Option<&str> {
        self.ordered_names.first().map(String::as_str)
    }

    pub fn categories(&self) -> &[SketchCategory] {
        &self.categories
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::GraphBuilder;
    use crate::sketch::Sketch;

    use super::*;

    struct TestSketch;

    impl Sketch for TestSketch {
        fn setup(&self, _graph: &mut GraphBuilder) {}
    }

    static CONFIG: SketchConfig = SketchConfig {
        name: "test",
        display_name: "Test",
        fps: 60.0,
        w: 640,
        h: 480,
        banks: 4,
    };

    #[test]
    fn registry_registers_and_lists_names() {
        let mut registry = RuntimeRegistry::new();
        registry
            .register(&CONFIG, || Box::new(TestSketch))
            .expect("register test sketch");

        assert_eq!(registry.sketch_names(), &["test"]);
        assert_eq!(registry.first_sketch_name(), Some("test"));
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn registry_rejects_category_with_unknown_sketch() {
        let mut registry = RuntimeRegistry::new();
        registry
            .register(&CONFIG, || Box::new(TestSketch))
            .expect("register test sketch");

        let err = registry
            .define_category(
                "bad",
                true,
                vec!["test".to_string(), "missing".to_string()],
            )
            .expect_err("category should fail when sketch is unknown");

        assert!(err.contains("unknown sketch"));
    }

    #[test]
    fn registry_stores_categories() {
        let mut registry = RuntimeRegistry::new();
        registry
            .register(&CONFIG, || Box::new(TestSketch))
            .expect("register test sketch");
        registry
            .define_category("main", true, vec!["test".to_string()])
            .expect("define category");

        assert_eq!(registry.categories().len(), 1);
        assert_eq!(registry.categories()[0].title, "main");
        assert!(registry.categories()[0].enabled);
        assert_eq!(registry.categories()[0].sketches, vec!["test"]);
    }
}
