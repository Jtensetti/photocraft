#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir public/pkg target/wasm32-unknown-unknown/release/creative_studio.wasm
