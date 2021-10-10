#![feature(bool_to_option)]
pub mod world;
pub mod wasm_helpers;

use world::{Direction, World};

pub trait Hero {
    fn spawn() -> Self;
    fn act(&mut self, world: &impl World) -> Action;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Move(Direction),
    StayStill,
}
