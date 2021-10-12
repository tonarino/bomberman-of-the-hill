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
