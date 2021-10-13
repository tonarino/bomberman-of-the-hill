use bomber_lib::{
    self,
    world::{Direction, Tile, World},
    Action, Player,
};
use bomber_macro::wasm_player;
use strum::IntoEnumIterator;

/// To build a `wasm player`, all that's needed is to implement the
/// `Player` trait, which defines how the player interacts with the
/// world, and to mark the struct with the `wasm_player` attribute,
/// which exposes the `wasm` exports the game expects to hot-swap
/// the player in.
#[wasm_player]
struct Wanderer {
    preferred_direction: Direction,
}

impl Player for Wanderer {
    fn spawn() -> Self {
        Self {
            preferred_direction: Direction::North,
        }
    }
    fn act(&mut self, world: &impl World) -> Action {
        // A wanderer walks to their preferred direction if it's free.
        // If it isn't, they  walk to the first free tile they inspect.
        let tile_is_free = |d: &Direction| world.inspect(*d) == Tile::EmptyFloor;
        if tile_is_free(&self.preferred_direction) {
            Action::Move(self.preferred_direction)
        } else {
            Direction::iter()
                .find(tile_is_free)
                .map(Action::Move)
                .unwrap_or(Action::StayStill)
        }
    }
}
