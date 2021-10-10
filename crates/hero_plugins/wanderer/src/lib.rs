use std::sync::Mutex;

use hero_lib::{self, Action, Hero, world::{Direction, Tile, World}};
use strum::IntoEnumIterator;
use hero_macro::wasm_hero;

/// To build a `wasm hero`, all that's needed is to implement the
/// `Hero` trait, which defines how the hero interacts with the
/// world, and to mark the struct with the `wasm_hero` attribute,
/// which exposes the `wasm` exports the game expects to hot-swap
/// the hero in.
#[wasm_hero]
struct Wanderer {
    preferred_direction: Direction,
}

impl Hero for Wanderer {
    fn spawn() -> Self { Self { preferred_direction: Direction::North } }
    fn act(&self, world: &impl World) -> Action {
        // A wanderer walks to his preferred direction if it's free.
        // If it isn't, they  walk to the first free tile they inspect.
        let tile_is_free = |d: &Direction| world.inspect(*d) == Tile::EmptyFloor;
        if tile_is_free(&self.preferred_direction) {
            Action::Move(self.preferred_direction)
        } else {
            Direction::iter().filter(tile_is_free).next().map(Action::Move).unwrap_or(Action::StayStill)
        }
    }
}
