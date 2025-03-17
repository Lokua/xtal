start *ARGS:
  RUST_LOG=lattice=info cargo run --release {{ARGS}}

debug *ARGS:
  RUST_LOG=lattice=debug cargo run --release {{ARGS}}

instrument *ARGS:
  RUST_LOG=lattice=debug cargo run --release --features instrumentation {{ARGS}}

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
  RUST_LOG=lattice=trace cargo test --lib -- {{ARGS}}
  
test-debug *ARGS:
  RUST_LOG=lattice=debug cargo test --lib -- {{ARGS}}

test-verbose *ARGS:
  RUST_LOG=lattice=trace cargo test --lib --show-output -- {{ARGS}}  

bench *ARGS:
  cargo bench {{ARGS}}

docs:
  cargo doc --open --release --document-private-items --no-deps -p lattice

stats:
  sccache --show-stats

list_midi_ports:
  RUST_LOG=lattice=debug cargo run --release --bin list_midi_ports

vmc:
  RUST_LOG=lattice=info,virtual_midi_controller=info cargo run --release \
  --bin virtual_midi_controller

debug-vmc:
  RUST_LOG=lattice=debug,virtual_midi_controller=debug cargo run --release \
  --bin virtual_midi_controller

# ------------------------------------------------------------------------------
#  Scripts
# ------------------------------------------------------------------------------
md:
  node scripts/image-markdown.mjs

gen *ARGS:
  node scripts/gen-uniforms.mjs {{ARGS}}

unpack *ARGS:
  node scripts/unpack.mjs {{ARGS}}
