use xtal2::prelude::*;

mod sketches;
use sketches::main::grid_splash_bw;
use sketches::templates::{basic, compute, feedback, image, multipass};

fn main() {
    let registry = xtal2::register_sketches! {
        {
            title: "Main",
            enabled: true,
            sketches: [
                grid_splash_bw,
            ]
        },
        {
            title: "Templates",
            enabled: true,
            sketches: [
                basic,
                feedback,
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

    if let Err(err) = run_registry(registry, initial_sketch.as_deref()) {
        eprintln!("xtal2 runtime failed: {}", err);
        std::process::exit(1);
    }
}
