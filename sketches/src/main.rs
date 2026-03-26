use xtal::prelude::*;

mod constants;
mod core;
mod dev;
mod drafts;
mod templates;
use core::*;
use dev::*;
use drafts::*;
use templates::*;

fn main() {
    let registry = xtal::register_sketches! {
        {
            title: "Main",
            enabled: true,
            sketches: [
                acc,
                blob,
                cloud,
                d_warp,
                dreams,
                dyn_uni,
                flow,
                grid_splash,
                gyroid,
                hatch,
                interference,
                ink,
                marcher,
                neural,
                rm,
                rm_auto,
                sline,
                spiral,
                un,
                watercolor,
                wave_sphere,
                wave_fract,
            ]
        },
        {
            title: "Drafts",
            enabled: true,
            sketches: [
                displ,
                grid_splash_bw,
                layers,
                phase_matrix,
            ]
        },
        {
            title: "Dev",
            enabled: true,
            sketches: [
                animation_dev,
                clock_dev,
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
