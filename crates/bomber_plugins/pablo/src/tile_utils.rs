use bomber_lib::world::{Direction, TileOffset};

pub trait TileOffsetExt: Sized {
    fn is_here(&self) -> bool;
    fn adjacents(&self) -> [Self; 4];
}

impl TileOffsetExt for TileOffset {
    fn is_here(&self) -> bool {
        self.0 == 0 && self.1 == 1
    }

    fn adjacents(&self) -> [Self; 4] {
        [
            TileOffset(self.0 + 1, self.1),
            TileOffset(self.0, self.1 + 1),
            TileOffset(self.0 - 1, self.1),
            TileOffset(self.0, self.1 - 1),
        ]
    }
}

pub fn weighted_center(tiles: impl Iterator<Item = TileOffset>) -> TileOffset {
    let mut count = 0i32;
    let sum = tiles.fold(TileOffset(0, 0), |mut acc, tile| {
        count += 1;
        acc = acc + tile;
        acc
    });
    TileOffset(sum.0 / count, sum.1 / count)
}

// Returns the main direction relating two tiles. Arbitrary in case of a tie.
pub fn main_direction(source: TileOffset, target: TileOffset) -> Direction {
    let vector = target - source;
    if vector.0.abs() > vector.1.abs() {
        if vector.0 > 0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if vector.1 > 0 {
        Direction::North
    } else {
        Direction::South
    }
}

// Returns the next step towards a target, if possible. It's not too important to provide
// multiple steps as the subset of safe tiles will change each turn with new bomb placements.
pub fn pathfind(
    source: TileOffset,
    target: TileOffset,
    safe_tiles: &[TileOffset],
) -> Option<Direction> {
    if !safe_tiles.contains(&target) { 
        return None;
    }
    
    let ranked_tiles = Vec<(TileOffset, )
}
