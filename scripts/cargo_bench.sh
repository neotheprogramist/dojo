#!/bin/bash
set -euxo pipefail

# Can be run for one intergration test with: `--test TEST_NAME`

# prepare contract
sozo --manifest-path crates/benches/contracts/Scarb.toml build
sozo --manifest-path crates/benches/contracts/Scarb.toml migrate apply --rpc-url http://localhost:5050

#run bench and show results
rm -f crates/benches/gas_usage.txt
cargo test bench $@ -- --ignored
cargo run --bin benches crates/benches/gas_usage.txt
