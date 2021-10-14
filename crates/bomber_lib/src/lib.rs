pub mod world;

use world::{Direction, RelativePosition, Tile};
use serde::{Serialize, Deserialize};
pub use bincode;

pub trait Player: Default {
    fn act(&mut self, surroundings: Vec<(Tile, RelativePosition)>) -> Action;
    fn name(&self) -> String;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Move(Direction),
    StayStill,
}
