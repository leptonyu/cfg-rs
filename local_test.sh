#!/bin/bash

set -e

cargo version
cargo build --verbose

cargo clippy -- -D warnings

cargo test --verbose --workspace
cargo test --verbose --workspace --all-features
cargo test --verbose --workspace --lib ## Default feature
cargo test --verbose --workspace --lib --no-default-features 
cargo test --verbose --workspace --lib --no-default-features --features=toml
cargo test --verbose --workspace --lib --no-default-features --features=yaml
cargo test --verbose --workspace --lib --no-default-features --features=rand
cargo run --example simple --features full
cargo run --example test_suit
cargo run --example salak
cargo run --example thread_pool
cargo run --example logger --features full
cargo run --example refresh
cargo run --example watch --features yaml
cargo run --example profile --features toml
cargo run --example validate --features regex