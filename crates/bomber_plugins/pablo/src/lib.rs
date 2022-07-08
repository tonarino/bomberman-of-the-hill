use anyhow::{anyhow, Result};
use std::cmp::{max, min};
use tile_utils::weighted_center;

use bomber_lib::{
    self,
    world::{Enemy, Object, Ticks, Tile, TileOffset},
    Action, Player,
};
use bomber_macro::wasm_export;

mod tile_utils;

type FullTile = (Tile, Option<Object>, Option<Enemy>, TileOffset);
type Bomb = (Ticks, u32, TileOffset);

#[derive(Default)]
struct Bomber;

fn bombs(surroundings: &[FullTile]) -> Vec<Bomb> {
    surroundings
        .iter()
        .cloned()
        .filter_map(|(_, obj, _, offset)| match obj {
            Some(Object::Bomb { fuse_remaining, range }) => Some((fuse_remaining, range, offset)),
            _ => None,
        })
        .collect()
}

fn empty_tiles(surroundings: &[FullTile]) -> Vec<TileOffset> {
    surroundings.iter()
        // Filter out any tiles with solid objects
        .filter(|(_, object, _, _)| !matches!(object, Some(o) if o.is_solid()))
        // Filter out any tiles with enemies
        .filter(|(_, _, enemy, _)| !enemy.is_some())
        // Filter out any otherwise unwalkable tiles
        .filter(|(tile, _, _, _)| !matches!(tile, Tile::Wall))
        .map(|(_, _, _, offset)| *offset)
        .collect::<Vec<_>>()
}

// // Returns the vector of tiles that it's safe to stand on this turn
fn safe_subset(surroundings: &[FullTile]) -> Vec<TileOffset> {
    let bombs = bombs(surroundings);
    let empty_tiles = empty_tiles(surroundings);
    let mut bombs_about_to_explode: Vec<Bomb> =
        bombs.iter().cloned().filter(|(Ticks(t), _, _)| *t == 0).collect();

    // iteratively add all bombs that will be triggered by bombs about to explode.
    iterative_explosions(&mut bombs_about_to_explode, &bombs, &empty_tiles);

    empty_tiles
        .iter()
        .filter(|offset| {
            bombs_about_to_explode.iter().any(|(_, range, bomb_offset)| {
                in_range_of_bomb(**offset, *bomb_offset, *range, &empty_tiles)
            })
        })
        .cloned()
        .collect()
}

fn iterative_explosions(
    bombs_about_to_explode: &mut Vec<Bomb>,
    bombs: &[Bomb],
    empty_tiles: &[TileOffset],
) {
    loop {
        let bombs_about_to_explode_before = bombs_about_to_explode.clone();
        bombs_about_to_explode.extend(
            bombs
                .iter()
                .cloned()
                .filter(|(_, _, offset)| {
                    !bombs_about_to_explode_before.iter().any(|(_, _, o)| o == offset)
                        && bombs_about_to_explode_before
                            .iter()
                            .any(|(_, r, o)| in_range_of_bomb(*offset, *o, *r, &empty_tiles))
                })
                .map(|(fuse, range, offset)| (fuse, range, offset)),
        );
        if bombs_about_to_explode.len() == bombs_about_to_explode_before.len() {
            break;
        };
    }
}

fn tiles_between(source: TileOffset, target: TileOffset) -> Result<Vec<TileOffset>> {
    if source.0 == target.0 {
        Ok((min(source.1, target.1)..max(source.1, target.1))
            .skip(1)
            .map(move |i| TileOffset(source.0, i))
            .collect())
    } else if source.1 == target.1 {
        Ok((min(source.0, target.0)..max(source.0, target.0))
            .skip(1)
            .map(move |i| TileOffset(i, source.1))
            .collect())
    } else {
        Err(anyhow!("Tiles are not orthogonal"))
    }
}

fn in_range_of_bomb(
    position: TileOffset,
    bomb_position: TileOffset,
    bomb_range: u32,
    empty_tiles: &[TileOffset],
) -> bool {
    if let Ok(tiles) = tiles_between(position, bomb_position) {
        tiles.len() <= bomb_range as usize && !tiles.iter().any(|t| !empty_tiles.contains(t))
    } else {
        false
    }
}

impl Bomber {}

#[wasm_export]
impl Player for Bomber {
    #[allow(clippy::empty_loop)]
    fn act(
        &mut self,
        surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, TileOffset)>,
    ) -> Action {
        let safe_tiles = safe_subset(&surroundings);
        let hill_center = weighted_center(
            surroundings.iter().filter_map(|(t, _, _, o)| matches!(t, Tile::Hill).then_some(*o)),
        );
        todo!()
    }

    fn name(&self) -> String {
        "unsafe{!}".into()
    }

    fn team_name() -> String {
        "Asbestos".into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn tiles_between_in_orthogonal_direction() {
        assert_eq!(
            tiles_between(TileOffset(3, 3), TileOffset(5, 3)).unwrap(),
            vec![TileOffset(4, 3)]
        );
        assert_eq!(
            tiles_between(TileOffset(3, 8), TileOffset(3, 3)).unwrap(),
            vec![TileOffset(3, 4), TileOffset(3, 5), TileOffset(3, 6), TileOffset(3, 7),]
        );
        assert!(tiles_between(TileOffset(3, 8), TileOffset(4, 5)).is_err());
        assert_eq!(
            tiles_between(TileOffset(3, 8), TileOffset(3, 8)).unwrap(),
            vec![] // identical
        );
    }
}
