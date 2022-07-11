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

pub fn closest_to(
    target: TileOffset,
    tiles: impl Iterator<Item = TileOffset>,
) -> Option<TileOffset> {
    tiles
        .fold((u32::MAX, None), |(shortest_distance, closest_tile), tile| {
            let distance = (target - tile).taxicab_distance();
            if distance < shortest_distance {
                (distance, Some(tile))
            } else {
                (shortest_distance, closest_tile)
            }
        })
        .1
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
pub fn pathfind(target: TileOffset, safe_tiles: &[TileOffset]) -> Option<Direction> {
    if !safe_tiles.contains(&target) {
        return None;
    }

    let mut path = vec![target];
    let mut new_tiles = path.clone();
    let source = TileOffset(0, 0);

    while !new_tiles.is_empty() && path.iter().all(|t| *t != source) {
        new_tiles = new_tiles
            .iter()
            .flat_map(|tile| tile.adjacents().into_iter())
            .filter(|tile| safe_tiles.contains(tile) && !path.contains(tile))
            .collect();
        path.extend(new_tiles.iter());
    }

    path.iter().find(|t| source.adjacents().contains(t)).map(|t| main_direction(source, *t))
}
