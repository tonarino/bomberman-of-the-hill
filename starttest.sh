#!/bin/bash

cargo build -p pablo --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/pablo.wasm crates/bomber_game/assets/players/pablo.wasm
cp target/wasm32-unknown-unknown/release/pablo.wasm crates/bomber_game/assets/players/pablo2.wasm
cp target/wasm32-unknown-unknown/release/pablo.wasm crates/bomber_game/assets/players/pablo3.wasm
cp target/wasm32-unknown-unknown/release/pablo.wasm crates/bomber_game/assets/players/pablo4.wasm
cp target/wasm32-unknown-unknown/release/pablo.wasm crates/bomber_game/assets/players/pablo5.wasm
cargo run --release --bin bomber_game