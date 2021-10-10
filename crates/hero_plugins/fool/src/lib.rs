use std::sync::Mutex;

use hero_lib::{self, Action, Hero, world::{Direction, Tile, World}};
use rand::prelude::SliceRandom;
use hero_macro::wasm_hero;

/// To build a `wasm hero`, all that's needed is to implement the
/// `Hero` trait, which defines how the hero interacts with the
/// world, and to mark the struct with the `wasm_hero` attribute,
/// which exposes the `wasm` exports the game expects to hot-swap
/// the hero in.
#[wasm_hero]
struct Fool;

impl Hero for Fool {
    fn spawn() -> Self { Self }
    fn act(&self, _: &impl World) -> Action {
        // A fool just ignores the world and travels north! Or somewhere
        // close to north! What's the worst that could happen?
        let possible_moves = [Direction::North, Direction::NorthWest, Direction::NorthEast];
        Action::Move(*possible_moves.choose(&mut rand::thread_rng()).unwrap())
    }
}
