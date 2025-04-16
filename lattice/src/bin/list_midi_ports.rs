use lattice::internal::midi::print_ports;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    print_ports()
}
