use xtal2::prelude::*;

mod sketches;
use sketches::{compute, demo, image, multipass};

fn main() {
    let registry = xtal2::register_sketches! {
        {
            title: "Main",
            enabled: true,
            sketches: [
                demo,
                multipass,
                compute,
                image,
            ]
        },
    }
    .unwrap_or_else(|err| {
        eprintln!("xtal2 sketch registry failed: {}", err);
        std::process::exit(1);
    });

    let initial_sketch = std::env::args().nth(1);

    if let Err(err) =
        run_registry_with_web_view(registry, initial_sketch.as_deref())
    {
        eprintln!("xtal2 runtime failed: {}", err);
        std::process::exit(1);
    }
}
