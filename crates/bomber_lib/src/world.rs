use std::convert::TryFrom;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

pub trait World {
    fn inspect(&self, direction: Direction) -> Tile;
}

#[derive(EnumIter, Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    West,
    North,
    East,
    South,
}

#[derive(EnumIter, Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Tile {
    Wall,
    Lava,
    Switch,
    EmptyFloor,
}

/// Position relative to something (typically the player, as this type is used
/// to tell the player where tiles are respective to them, without leaking the
/// map layout).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Distance(pub i32, pub i32);

impl Distance {
    /// Whether the position represents a tile adjacent to the origin.
    pub fn adjacent(&self) -> bool {
        self.0.abs() <= 1 && self.1.abs() <= 1
    }

    /// Whether the position represents a tile orthogonally adjacent
    /// to the origin (no diagonals)
    pub fn orthogonally_adjacent(&self) -> bool {
        (self.0.abs() == 1 && self.1 == 0) || (self.0 == 0 && self.1.abs() == 1)
    }
}

impl std::ops::Add for Distance {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl<T: Into<i32>> std::ops::Mul<T> for Direction {
    type Output = Distance;

    fn mul(self, rhs: T) -> Self::Output {
        let distance = rhs.into();
        match self {
            Direction::West => Distance(-distance, 0),
            Direction::North => Distance(0, distance),
            Direction::East => Distance(distance, 0),
            Direction::South => Distance(0, -distance),
        }
    }
}

/// Quality of life conversion for the player to simplify their navigation logic.
impl TryFrom<Distance> for Direction {
    // TODO proper error return
    type Error = ();

    fn try_from(p: Distance) -> Result<Self, ()> {
        match p {
            Distance(x, 0) if x > 0 => Ok(Direction::East),
            Distance(x, 0) if x < 0 => Ok(Direction::West),
            Distance(0, y) if y > 0 => Ok(Direction::North),
            Distance(0, y) if y < 0 => Ok(Direction::South),
            _ => Err(()),
        }
    }
}
