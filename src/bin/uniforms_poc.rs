use nannou::geom::Rect;
use nannou::glam::vec2;
use std::error::Error;

use derives::uniforms;
use lattice::framework::prelude::*;

#[uniforms(banks = 5)]
struct Foo {}

fn main() -> Result<(), Box<dyn Error>> {
    let x = Foo::default();
    println!("a: {:?}", x.a);
    println!("e: {:?}", x.e);
    println!();

    let hub = ControlHubBuilder::new()
        .timing(ManualTiming::new(Bpm::new(120.0)))
        .slider_n("a3", 0.5)
        .slider_n("e2", 0.5)
        .build();

    let wr = WindowRect::new(Rect::from_wh(vec2(500.0, 300.0)));

    let mut x = Foo::from((&wr, &hub));
    x.set("b4", 99.0);
    println!("a: {:?}", x.a);
    println!("b: {:?}", x.b);
    println!("e: {:?}", x.e);

    Ok(())
}
