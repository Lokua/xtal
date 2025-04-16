use std::error::Error;

use lattice::midi::print_ports;

fn main() -> Result<(), Box<dyn Error>> {
    print_ports()
}
