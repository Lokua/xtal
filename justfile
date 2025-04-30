start *ARGS:
  RUST_LOG=xtal=info,sketches=info cargo run --release {{ARGS}}

debug *ARGS:
  RUST_LOG=xtal=debug,sketches=debug cargo run --release {{ARGS}}

ui:
  bun --cwd xtal-ui start

instrument *ARGS:
  RUST_LOG=xtal=debug cargo run --release --features instrumentation {{ARGS}}

trace *ARGS:
  RUST_LOG=xtal=trace,sketches=trace cargo run --release {{ARGS}}

# Usage: just trace-module framework::frame_controller <sketch>
trace-module MODULE *ARGS:
  RUST_LOG=xtal=info,xtal::{{MODULE}}=trace cargo run --release {{ARGS}}

# Usage: just trace-module framework::frame_controller <sketch>
debug-module MODULE *ARGS:
  RUST_LOG=xtal=info,xtal::{{MODULE}}=debug cargo run --release {{ARGS}}

# To test just a single test, past the test name e.g. just test my_test
# To test a single module, pass the module name e.g. just test my::module
test *ARGS:
  RUST_LOG=xtal=trace cargo test --lib --package xtal -- {{ARGS}}
  
test-debug *ARGS:
  RUST_LOG=xtal=debug cargo test --lib --package xtal -- {{ARGS}}

test-verbose *ARGS:
  RUST_LOG=xtal=trace cargo test --lib --package xtal --show-output -- {{ARGS}}  

bench *ARGS:
  cargo bench {{ARGS}}

docs-internal:
  cargo doc --package xtal --document-private-items --open
  
docs:
  cargo doc --package xtal --open

stats:
  sccache --show-stats

list_midi_ports:
  RUST_LOG=xtal=debug cargo run --release --bin list_midi_ports

# ------------------------------------------------------------------------------
#  Scripts
# ------------------------------------------------------------------------------
md:
  node scripts/image-markdown.mjs

gen *ARGS:
  node scripts/gen-uniforms.mjs {{ARGS}}

unpack *ARGS:
  node scripts/unpack.mjs {{ARGS}}
