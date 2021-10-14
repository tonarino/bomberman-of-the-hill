pub mod world;

use world::{Direction, RelativePosition, Tile};
use bomber_macro::wasm_wrap;
use wasmtime::AsContextMut;

// Reexports for quality of life when using the wasm macros
pub use serde::{Serialize, Deserialize};
pub use bincode;
pub use wasmtime;
pub use anyhow;

#[wasm_wrap]
pub trait Player: Default {
    fn act(&mut self, surroundings: Vec<(Tile, RelativePosition)>, result: LastTurnResult) -> Action;
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
}
