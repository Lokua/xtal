pub mod config;
pub mod framework;
pub mod runtime;
mod sketches;

fn main() {
    runtime::app::run();
}
