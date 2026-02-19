use xtal::prelude::*;

mod constants;
mod core;
mod templates;
use core::*;
use templates::*;

fn main() {
    let registry = xtal::register_sketches! {
        {
            title: "Main",
            enabled: true,
            sketches: [
                grid_splash_bw,
                watercolor,
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
        eprintln!("xtal sketch registry failed: {}", err);
        std::process::exit(1);
    });

    let initial_sketch = std::env::args().nth(1);

    if let Err(err) = run_registry(registry, initial_sketch.as_deref()) {
        eprintln!("xtal runtime failed: {}", err);
        std::process::exit(1);
    }
}
