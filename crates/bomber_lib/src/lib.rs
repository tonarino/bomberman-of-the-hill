pub mod world;

use bomber_macro::wasm_wrap;
#[cfg(not(target_family = "wasm"))]
use wasmtime::AsContextMut;

use world::{Direction, TileOffset, Tile};

// Reexports for quality of life when using the wasm macros
#[cfg(not(target_family = "wasm"))]
pub use anyhow;
pub use bincode;
pub use serde::{Deserialize, Serialize};
#[cfg(not(target_family = "wasm"))]
pub use wasmtime;

#[wasm_wrap]
pub trait Player: Default {
    fn act(&mut self, surroundings: Vec<(Tile, TileOffset)>, last_result: LastTurnResult) -> Action;
    fn name(&self) -> String;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Move(Direction),
    StayStill,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LastTurnResult {
    Moved(Direction),
    ActionFailed,
    Died,
    StoodStill,
}
