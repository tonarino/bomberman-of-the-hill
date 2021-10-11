# Wasm Hero

Simple experiments on hand-adapted Rust to Rust Wasm game plugins.

# Instructions

* Build either of the plugins with `cargo build -p fool --target
  wasm32-unknown-unknown` or `cargo build -p wanderer --target
  wasm32-unknown-unknown`. This will generate `.wasm` files under
  `target/wasm32-unknown-unknown/debug/`
* Launch the runner with `cargo run --release -p hero_runner`
* Drop either of the `wasm` files you generated in step 1 in
  `crates/hero_runner/assets/heroes`
* Watch the heroes go!
