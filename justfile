start *ARGS:
  RUST_LOG=lattice=info cargo run --release {{ARGS}}

debug *ARGS:
  RUST_LOG=lattice=debug cargo run --release {{ARGS}}

trace *ARGS:
  RUST_LOG=lattice=trace cargo run --release {{ARGS}}

# Usage: just trace-module framework::frame_controller <sketch>
trace-module MODULE *ARGS:
  RUST_LOG=lattice=info,lattice::{{MODULE}}=trace cargo run --release {{ARGS}}

# To test just a single test, past the test name e.g. just test my_test
# To test a single module, pass the module name e.g. just test my::module
test *ARGS:
  RUST_LOG=lattice=trace cargo test -- {{ARGS}}
  
test-debug *ARGS:
  RUST_LOG=lattice=trace cargo test -- {{ARGS}}

generate-markdown-index:
  cargo run -p image-markdown --release

md:
  just generate-markdown-index

stats:
  sccache --show-stats
