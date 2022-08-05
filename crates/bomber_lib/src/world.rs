use std::convert::TryFrom;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(
    EnumIter, Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Direction {
    West,
    North,
    East,
    South,
}

#[derive(
    EnumIter, Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Tile {
    /// Impassable terrain that cannot be blown up with bombs.
    Wall,
    /// Walkable terrain.
    Floor,
    /// Stand on this terrain to gain victory points!
    Hill,
}

/// Anything that can be found on top of a tile, other than a player.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Object {
    /// A ticking bomb placed by a player.
    Bomb {
        /// How many turns are left until it explodes.
        fuse_remaining: Ticks,
        /// How many tiles it can reach when it explodes.
        range: u32,
    },
    /// An item that improves one of your abilities.
    PowerUp(PowerUp),
    /// An explodable crate. Will stop the progress of bomb explosions,
    /// and it may contain powerups!
    Crate,
}

/// A rival player.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Enemy {
    pub name: String,
    pub team_name: String,
    pub score: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PowerUp {
    /// Each increases the range of your bomb explosions by one.
    BombRange,
    /// Each increases the number of ticking bombs you can place on the world at once.
    SimultaneousBombs,
    /// Each increases the distance that your character can see every turn.
    VisionRange,
}

impl Object {
    pub fn is_solid(&self) -> bool {
        match self {
            Object::Bomb { .. } | Object::Crate => true,
            Object::PowerUp(_) => false,
        }
    }
}

impl PowerUp {
    pub const fn max_count_per_player(&self) -> u32 {
        match self {
            PowerUp::BombRange => 5,
            PowerUp::SimultaneousBombs => 3,
            PowerUp::VisionRange => 5,
        }
    }
}

/// Ticks measure game time. Players make one decision per tick.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Ticks(pub u32);

/// Position relative to something (typically you),
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    pub fn all() -> [Direction; 4] {
        [Direction::West, Direction::North, Direction::East, Direction::South]
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
impl std::ops::Sub for TileOffset {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

/// Quality of life conversion.
impl TryFrom<TileOffset> for Direction {
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
