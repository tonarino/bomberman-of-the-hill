use strum_macros::EnumIter;

pub trait World {
    fn inspect(&self, direction: Direction) -> Tile;
}

#[derive(EnumIter, Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    West,
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
}

#[derive(EnumIter, Copy, Clone, Debug, Eq, PartialEq)]
pub enum Tile {
    Wall,
    Lava,
    Switch,
    EmptyFloor,
}

impl From<char> for Tile {
    fn from(character: char) -> Self {
        match character {
            '.' => Tile::EmptyFloor,
            '#' => Tile::Wall,
            'X' => Tile::Lava,
            's' => Tile::Switch,
            _ => panic!("Character has no associated tile"),
        }
    }
}
