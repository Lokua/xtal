#!/bin/bash

# Clean previous criterion data to avoid confusion
echo "Cleaning previous benchmark data..."
rm -rf target/criterion

# Run the lock implementation benchmark as baseline
echo "Running lock implementation benchmark..."
cargo bench --bench frame_controller_bench -- --save-baseline frame_controller_lock_impl

# Run the atomic implementation benchmark with comparison
echo "Running atomic implementation benchmark..."
cargo bench --bench frame_controller_bench --features atomic_frames -- --baseline frame_controller_lock_impl

echo "Done. Check the Criterion report at target/criterion/report/index.html"

open ./target/criterion/report/index.html