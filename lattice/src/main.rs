use std::error::Error;

use lattice::internal::run_web_view;

fn main() -> Result<(), Box<dyn Error>> {
    run_web_view()
}
