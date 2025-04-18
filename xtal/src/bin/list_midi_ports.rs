use std::error::Error;
use xtal::internal::midi::print_ports;

fn main() -> Result<(), Box<dyn Error>> {
    print_ports()
}
