use xtal2::graph::GraphBuilder;
use xtal2::prelude::*;

mod demo {
    use super::*;

    pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
        name: "demo_p2",
        display_name: "Demo P2",
        fps: 60.0,
        w: 640,
        h: 480,
        banks: 4,
    };

    pub struct DemoSketch;

    impl Sketch for DemoSketch {
        fn setup(&self, _graph: &mut GraphBuilder) {}
    }

    pub fn init() -> DemoSketch {
        DemoSketch
    }
}

mod image {
    use super::*;

    pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
        name: "image_p2",
        display_name: "Image P2",
        fps: 60.0,
        w: 640,
        h: 480,
        banks: 4,
    };

    pub struct ImageSketch;

    impl Sketch for ImageSketch {
        fn setup(&self, _graph: &mut GraphBuilder) {}
    }

    pub fn init() -> ImageSketch {
        ImageSketch
    }
}

#[test]
fn register_sketches_macro_builds_registry_with_categories() {
    let registry = register_sketches! {
        {
            title: "Main",
            enabled: true,
            sketches: [demo]
        },
        {
            title: "Media",
            enabled: false,
            sketches: [image]
        },
    }
    .expect("macro should build runtime registry");

    assert_eq!(registry.sketch_names(), &["demo_p2", "image_p2"]);
    assert_eq!(registry.categories().len(), 2);
    assert!(registry.categories()[0].enabled);
    assert!(!registry.categories()[1].enabled);
}

#[test]
fn sketch_assets_resolves_default_and_custom_paths() {
    let assets = SketchAssets::from_file("src/sketches/demo.rs");
    assert!(assets.wgsl().ends_with("src/sketches/demo.wgsl"));
    assert!(assets.yaml().ends_with("src/sketches/demo.yaml"));

    let custom = SketchAssets::with_stem("src/sketches/demo.rs", "other");
    assert!(custom.wgsl().ends_with("src/sketches/other.wgsl"));
}
