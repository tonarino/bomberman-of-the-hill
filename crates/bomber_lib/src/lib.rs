pub mod world;

use bomber_macro::wasm_wrap;
#[cfg(not(target_family = "wasm"))]
use wasmtime::AsContextMut;

use world::{Direction, Object, Tile, TileOffset};

// Reexports for quality of life when using the wasm macros
#[cfg(not(target_family = "wasm"))]
pub use anyhow;
pub use bincode;
pub use serde::{Deserialize, Serialize};
#[cfg(not(target_family = "wasm"))]
pub use wasmtime;

#[wasm_wrap]
pub trait Player: Default {
    fn act(
        &mut self,
        surroundings: Vec<(Tile, Option<Object>, TileOffset)>,
        last_result: LastTurnResult,
    ) -> Action;
    fn name(&self) -> String;
    fn team_name() -> String;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Move(Direction),
    StayStill,
    /// Places a bomb at the player's current location.
    DropBomb,
    /// Places a bomb at the player's current location while moving.
    DropBombAndMove(Direction),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LastTurnResult {
    Moved(Direction),
    ActionFailed,
    Died,
    StoodStill,
}
