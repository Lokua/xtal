start *ARGS:
  RUST_LOG=lattice=info cargo run --release {{ARGS}}

debug *ARGS:
  RUST_LOG=lattice=debug cargo run --release {{ARGS}}

test *ARGS:
  RUST_LOG=lattice=trace cargo test {{ARGS}}