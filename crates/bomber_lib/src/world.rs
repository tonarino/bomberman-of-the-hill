use std::convert::TryFrom;
use anyhow::{anyhow, Error};

use strum_macros::EnumIter;
use serde::{Serialize, Deserialize};

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
pub struct RelativePosition(pub i32, pub i32);

impl RelativePosition {
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

impl std::ops::Add for RelativePosition {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self (self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl<T: Into<i32>> std::ops::Mul<T> for Direction {
    type Output = RelativePosition;

    fn mul(self, rhs: T) -> Self::Output {
        let distance = rhs.into();
        match self {
            Direction::West => RelativePosition(-distance, 0),
            Direction::North => RelativePosition(0, distance),
            Direction::East => RelativePosition(distance, 0),
            Direction::South => RelativePosition(0, -distance),
        }
    }
}

/// Quality of life conversion for the player to simplify their navigation logic.
impl TryFrom<RelativePosition> for Direction {
    type Error = Error;

    fn try_from(p: RelativePosition) -> Result<Self, Self::Error> {
        match p {
            RelativePosition(x, 0) if x > 0 => Ok(Direction::East),
            RelativePosition(x, 0) if x < 0 => Ok(Direction::West),
            RelativePosition(0, y) if y > 0 => Ok(Direction::North),
            RelativePosition(0, y) if y < 0 => Ok(Direction::South),
            _ => Err(anyhow!("Relative Position does not correspond to an orthogonal direction")),
        }
    }
}
