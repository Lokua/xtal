use std::error::Error;

use derives::uniforms;

#[uniforms(count = 5)]
struct Foo {}

fn main() -> Result<(), Box<dyn Error>> {
    let x = Foo::default();
    println!("a: {:?}", x.a);
    println!("e: {:?}", x.e);

    let hub = ControlHubBuilder::new()
        .timing(ManualTiming::new(Bpm::new(120.0)))
        .slider_n("a1", 0.5)
        .slider_n("e2", 0.5)
        .build();

    let x = Foo::from_hub(&hub);
    println!("a: {:?}", x.a);
    println!("e: {:?}", x.e);

    Ok(())
}
