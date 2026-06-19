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
                blob,
                cloud,
                d_warp,
                dreams,
                dyn_uni,
                flow,
                grid_splash_2,
                grid_splash,
                gyroid,
                hatch_auto,
                interference,
                marcher,
                neural,
                rm,
                rm_auto,
                spiral,
                un,
                watercolor,
                wave_fract,
            ]
        },
        {
            title: "Drafts",
            enabled: true,
            sketches: [
                acc,
                domain_warps,
                displ,
                flow_snd,
                grid_splash_bw,
                hatch,
                ink,
                layers,
                phase_matrix,
                snd,
                sline,
                viaduct_poc,
                wave_sphere,
            ]
        },
        {
            title: "Dev",
            enabled: true,
            sketches: [
                animation_dev,
                clock_dev,
                projector_stress,
                video,
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
