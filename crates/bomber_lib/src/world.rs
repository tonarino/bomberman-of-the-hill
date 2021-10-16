use std::convert::TryFrom;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

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
    EmptyFloor,
    Hill,
}

#[derive(EnumIter, Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Object {
    Bomb,
    Crate,
}

/// Position relative to something (typically the player, as this type is used
/// to tell the player where tiles are respective to them, without leaking the
/// map layout).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TileOffset(pub i32, pub i32);

impl Direction {
    pub fn extend<T: Into<i32>>(&self, amount: T) -> TileOffset {
        match self {
            Direction::West => TileOffset(-amount.into(), 0),
            Direction::North => TileOffset(0, amount.into()),
            Direction::East => TileOffset(amount.into(), 0),
            Direction::South => TileOffset(0, -amount.into()),
        }
    }
}

impl TileOffset {
    /// Whether the position represents a tile adjacent to the origin.
    pub fn is_adjacent(&self) -> bool {
        self.0.abs() <= 1 && self.1.abs() <= 1
    }

    /// Whether the position represents a tile orthogonally adjacent
    /// to the origin (no diagonals)
    pub fn is_orthogonally_adjacent(&self) -> bool {
        (self.0.abs() == 1 && self.1 == 0) || (self.0 == 0 && self.1.abs() == 1)
    }

    pub fn taxicab_distance(&self) -> u32 {
        (self.0.abs() + self.1.abs()) as u32
    }

    pub fn chebyshev_distance(&self) -> u32 {
        self.0.abs().max(self.1.abs()) as u32
    }
}

impl std::ops::Add for TileOffset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

/// Quality of life conversion for the player to simplify their navigation logic.
impl TryFrom<TileOffset> for Direction {
    // TODO proper error return
    type Error = ();

    fn try_from(p: TileOffset) -> Result<Self, ()> {
        match p {
            TileOffset(x, 0) if x > 0 => Ok(Direction::East),
            TileOffset(x, 0) if x < 0 => Ok(Direction::West),
            TileOffset(0, y) if y > 0 => Ok(Direction::North),
            TileOffset(0, y) if y < 0 => Ok(Direction::South),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn various_distance_calculations() {
        let offset = TileOffset(4, 3);

        assert_eq!(offset.taxicab_distance(), 7);
        assert_eq!(offset.chebyshev_distance(), 4);
    }
}
