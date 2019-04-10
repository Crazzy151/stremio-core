#!/bin/sh
set -ex
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen ./target/wasm32-unknown-unknown/release/state_container_web.wasm --browser --out-dir ./build/release
