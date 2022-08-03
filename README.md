# Bomberman Of The Hill

Meetup attendees are tasked with training their bombermen to hold a hill. The
longer it's held, the bigger the score and bounty for taking them down!

# Preparation

Copy `.env.example` to `.env` and edit relevant variables.

# Instructions

Build either of the plugins with:

* `cargo build -p fool --release --target wasm32-unknown-unknown`

or

* `cargo build -p wanderer --release --target wasm32-unknown-unknown`

This will generate `.wasm` files under `target/wasm32-unknown-unknown/release/`

* Launch the runner with `cargo run --release -p bomber_game`
* Drop either of the `wasm` files you generated in step 1 in `rounds/1/`
* Watch the bombers go!

Run the upload server using `cargo run -p upload_server`
