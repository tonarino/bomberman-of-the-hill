use std::{convert::TryFrom, ops::Add};

use anyhow::{anyhow, Result};
use bomber_lib::world::{Direction, Tile};

use crate::Wrapper;

pub const INITIAL_LOCATION: Location = Location(4, 0);

#[allow(unused)]
#[rustfmt::skip]
pub const EASY: &str =
    "###...###\n\
     ##.....##\n\
     #..###..#\n\
     #..###..#\n\
     #...#...#\n\
     #.......#\n\
     ####.####";

#[allow(unused)]
#[rustfmt::skip]
pub const DANGEROUS: &str =
    "####.####\n\
     #.......#\n\
     #.#####.#\n\
     #.XXXXX.#\n\
     #.......#\n\
     #.......#\n\
     ####.####";

pub struct GameMap {
    tiles: Vec<Vec<Tile>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Location(pub usize, pub usize);

impl Add<Direction> for Location {
    type Output = Option<Location>;

    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::West | Direction::NorthWest | Direction::SouthWest if self.0 == 0 => None,
            Direction::South | Direction::SouthWest | Direction::SouthEast if self.1 == 0 => None,
            Direction::West => Some(Location(self.0 - 1, self.1)),
            Direction::NorthWest => Some(Location(self.0 - 1, self.1 + 1)),
            Direction::North => Some(Location(self.0, self.1 + 1)),
            Direction::NorthEast => Some(Location(self.0 + 1, self.1 + 1)),
            Direction::East => Some(Location(self.0 + 1, self.1)),
            Direction::SouthEast => Some(Location(self.0 + 1, self.1 - 1)),
            Direction::South => Some(Location(self.0, self.1 - 1)),
            Direction::SouthWest => Some(Location(self.0 - 1, self.1 - 1)),
        }
    }
}

impl GameMap {
    pub fn size(&self) -> (usize, usize) {
        (self.tiles[0].len(), self.tiles.len())
    }

    pub fn tile(&self, location: Location) -> Option<Tile> {
        self.tiles
            .get(location.1)
            .and_then(|v| v.get(location.0))
            .cloned()
    }

    /// When inspecting, out of bound tiles are considered to be walls. This simplifies
    /// the Wasm API for now, but it should probably be replaced as this matures (otherwise
    /// we're treating the wall as a sentinel value, and we can do better in Rust...)
    pub fn inspect_from(&self, location: Location, direction: Direction) -> Tile {
        (location + direction)
            .and_then(|p| self.tile(p))
            .unwrap_or(Tile::Wall)
    }
}

impl TryFrom<char> for Wrapper<Tile> {
    type Error = anyhow::Error;

    fn try_from(character: char) -> Result<Self, Self::Error> {
        match character {
            '.' => Ok(Wrapper(Tile::EmptyFloor)),
            '#' => Ok(Wrapper(Tile::Wall)),
            'X' => Ok(Wrapper(Tile::Lava)),
            's' => Ok(Wrapper(Tile::Switch)),
            _ => Err(anyhow!("Invalid character for tile: {}", character)),
        }
    }
}

impl TryFrom<&str> for GameMap {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> Result<Self> {
        let lines: Vec<&str> = text.lines().rev().collect();
        if lines.windows(2).any(|w| w[0].len() != w[1].len()) {
            Err(anyhow!("Mismatched row sizes in the game map"))
        } else if lines.len() == 0 || lines[0].len() == 0 {
            Err(anyhow!("Game map must have at least a row and a column"))
        } else {
            let convert_line = |l: &str| -> Result<Vec<Tile>> {
                l.chars()
                    .map(|c| Wrapper::<Tile>::try_from(c).map(|w| w.0))
                    .collect()
            };
            let tiles: Result<Vec<Vec<Tile>>> = lines.into_iter().map(convert_line).collect();
            Ok(Self { tiles: tiles? })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_game_maps() {
        #[rustfmt::skip]
        let game_map_text =
            "####.###\n\
             #......#\n\
             #.####.#\n\
             #..##..#\n\
             #X.##..#\n\
             #......#\n\
             ####.###";
        let game_map = GameMap::try_from(game_map_text).unwrap();
        assert_eq!(game_map.size(), (8, 7));
        assert_eq!(game_map.tile(Location(0, 0)).unwrap(), Tile::Wall);
        assert_eq!(game_map.tile(Location(4, 0)).unwrap(), Tile::EmptyFloor);
        assert_eq!(game_map.tile(Location(1, 1)).unwrap(), Tile::EmptyFloor);
        assert_eq!(game_map.tile(Location(1, 2)).unwrap(), Tile::Lava);
        assert_eq!(game_map.tile(Location(8, 8)), None);
    }
}
