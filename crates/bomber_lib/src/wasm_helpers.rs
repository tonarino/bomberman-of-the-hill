//! This module provides utilities to make game types convertible
//! to and from wasm primitives.

use strum::IntoEnumIterator;

use crate::{
    world::{Direction, Tile},
    Action,
};

impl From<Action> for u32 {
    fn from(action: Action) -> Self {
        match action {
            Action::StayStill => 0,
            Action::Move(direction) => direction as u32 + 1,
        }
    }
}

impl From<u32> for Action {
    fn from(raw: u32) -> Self {
        match raw {
            0 => Action::StayStill,
            x => Action::Move(Direction::from(x.saturating_sub(1))),
        }
    }
}

impl From<u32> for Tile {
    fn from(raw: u32) -> Self {
        Self::iter().nth(raw as usize).expect("Invalid raw tile index")
    }
}

impl From<u32> for Direction {
    fn from(raw: u32) -> Self {
        Self::iter().nth(raw as usize).expect("Invalid raw direction index")
    }
}
