start *ARGS:
  RUST_LOG=lattice=info cargo run --release {{ARGS}}

debug *ARGS:
  RUST_LOG=lattice=debug cargo run --release {{ARGS}}

trace *ARGS:
  RUST_LOG=lattice=trace cargo run --release {{ARGS}}

# Usage: just trace-module framework::frame_controller <sketch>
trace-module MODULE *ARGS:
  RUST_LOG=lattice=info,lattice::{{MODULE}}=trace cargo run --release {{ARGS}}

# Usage: just trace-module framework::frame_controller <sketch>
debug-module MODULE *ARGS:
  RUST_LOG=lattice=info,lattice::{{MODULE}}=debug cargo run --release {{ARGS}}

# To test just a single test, past the test name e.g. just test my_test
# To test a single module, pass the module name e.g. just test my::module
test *ARGS:
  RUST_LOG=lattice=trace cargo test -- {{ARGS}}
  
test-debug *ARGS:
  RUST_LOG=lattice=debug cargo test -- {{ARGS}}

test-verbose *ARGS:
  RUST_LOG=lattice=trace cargo test -- --show-output {{ARGS}}  

generate-markdown-index:
  cargo run -p image-markdown --release

md:
  just generate-markdown-index

open-docs:
  cargo doc --open --release --no-deps -p lattice

docs:
  cargo doc --release --no-deps -p derives -p lattice

stats:
  sccache --show-stats

bin *ARGS:
  RUST_LOG=lattice=debug cargo run --release --bin lattice_dynamic {{ARGS}}
