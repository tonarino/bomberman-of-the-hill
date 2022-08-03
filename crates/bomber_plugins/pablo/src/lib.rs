use anyhow::{anyhow, Result};
use bomber_lib::{
    self,
    world::{Direction, Enemy, Object, Ticks, Tile, TileOffset},
    Action, Player,
};
use bomber_macro::wasm_export;
use std::cmp::{max, min, Ordering};

mod tile_utils;

const TURN_LOOKAHEAD: usize = 3;

type FullTile = (Tile, Option<Object>, Option<Enemy>, TileOffset);
type Bomb = (Ticks, u32, TileOffset);

#[derive(Default)]
struct Bomber {
    boring_tiles: Vec<TileOffset>,
}

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

#[derive(Debug)]
struct SimulatedTurn {
    // If none, it means you died :(
    next_turn_surroundings: Option<Vec<FullTile>>,
}

#[derive(Clone, Debug)]
struct MultiTurnPlan {
    first_action: Action,
    final_surroundings: Vec<FullTile>,
}

impl MultiTurnPlan {
    fn continue_with(mut self, action: Action) -> Option<MultiTurnPlan> {
        let simulated_turn = simulate_turn(&self.final_surroundings, action);
        self.final_surroundings = simulated_turn.next_turn_surroundings?;
        Some(self)
    }

    fn final_tile(&self) -> &FullTile {
        self.final_surroundings
            .iter()
            .find(|(_, _, _, offset)| offset == &TileOffset(0, 0))
            .unwrap()
    }

    fn next_position(&self) -> TileOffset {
        match self.first_action {
            Action::Move(d) => d.extend(1),
            Action::DropBombAndMove(d) => d.extend(1),
            _ => TileOffset(0, 0),
        }
    }
}

fn all_possible_actions() -> impl Iterator<Item = Action> {
    [
        Action::DropBombAndMove(Direction::East),
        Action::DropBombAndMove(Direction::North),
        Action::DropBombAndMove(Direction::South),
        Action::DropBombAndMove(Direction::West),
        Action::Move(Direction::East),
        Action::Move(Direction::North),
        Action::Move(Direction::South),
        Action::Move(Direction::West),
        Action::StayStill,
        Action::DropBomb,
    ]
    .into_iter()
}

fn all_possible_plans(surroundings: &[FullTile]) -> Vec<MultiTurnPlan> {
    let mut plans = all_possible_actions()
            // We filter out standing bombs because it's a terrible idea in general.
        .filter(|a| a != &Action::DropBomb)
        .filter_map(|a| {
            let SimulatedTurn { next_turn_surroundings} =
                simulate_turn(surroundings, a);
            next_turn_surroundings.map(|s| MultiTurnPlan {
                first_action: a,
                final_surroundings: s,
            })
        })
        .collect::<Vec<_>>();

    for _ in 0..TURN_LOOKAHEAD {
        plans = plans
            .iter()
            .flat_map(|plan| {
                all_possible_actions().filter_map(|action| plan.clone().continue_with(action))
            })
            .collect();
    }

    plans
}

fn simulate_turn(surroundings: &[FullTile], action: Action) -> SimulatedTurn {
    // Modify surroundings with the immediate consequences of the player action, before starting the simulation
    let surroundings: Vec<_> = surroundings
        .iter()
        .cloned()
        .map(|(tile, object, enemy, offset)| match action {
            Action::DropBombAndMove(d) => (
                tile,
                if offset == TileOffset(0, 0) {
                    Some(object.unwrap_or(Object::Bomb { fuse_remaining: Ticks(2), range: 3 }))
                } else {
                    object
                },
                enemy,
                offset - d.extend(1),
            ),
            Action::DropBomb => (
                tile,
                if offset == TileOffset(0, 0) {
                    Some(object.unwrap_or(Object::Bomb { fuse_remaining: Ticks(2), range: 3 }))
                } else {
                    object
                },
                enemy,
                offset,
            ),
            Action::Move(d) => (tile, object, enemy, offset - d.extend(1)),
            _ => (tile, object, enemy, offset),
        })
        .collect();

    let bombs = dbg!(bombs(&surroundings));
    let empty_tiles = empty_tiles(&surroundings);
    let mut bombs_about_to_explode: Vec<Bomb> =
        bombs.iter().cloned().filter(|(Ticks(t), _, _)| *t == 0).collect();

    // iteratively add all bombs that will be triggered by bombs about to explode.
    iterative_explosions(&mut bombs_about_to_explode, &bombs, &empty_tiles);

    let mut safe_tiles = empty_tiles.iter().filter(|offset| {
        bombs_about_to_explode.iter().all(|(_, range, bomb_offset)| {
            !in_range_of_bomb(**offset, *bomb_offset, *range, &empty_tiles)
        })
    });

    if !safe_tiles.any(|t| t == &TileOffset(0, 0)) {
        return SimulatedTurn { next_turn_surroundings: None };
    }

    let next_turn_surroundings = surroundings
        .iter()
        .cloned()
        .map(|(tile, object, enemy, offset)| match object {
            // Clear bombs that are about to explode.
            Some(Object::Bomb { .. })
                if bombs_about_to_explode.iter().any(|(_, _, o)| *o == offset) =>
            {
                (tile, None, enemy, offset)
            },
            // Tick down the rest
            Some(Object::Bomb { fuse_remaining, range }) => (
                tile,
                Some(Object::Bomb {
                    fuse_remaining: Ticks(fuse_remaining.0.saturating_sub(1)),
                    range,
                }),
                enemy,
                offset,
            ),
            Some(Object::Crate)
                if bombs_about_to_explode.iter().any(|(_, range, bomb_offset)| {
                    in_range_of_bomb(offset, *bomb_offset, *range, &empty_tiles)
                }) =>
            {
                (tile, None, enemy, offset)
            },
            _ => (tile, object, enemy, offset),
        })
        .collect();
    SimulatedTurn { next_turn_surroundings: Some(next_turn_surroundings) }
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
                            .any(|(_, r, o)| in_range_of_bomb(*offset, *o, *r, empty_tiles))
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
        // Precalculate all viable plan N turns ahead (obviously limited by our current line of sight and
        // understanding). This includes only plans that don't get us killed!
        let mut plans = all_possible_plans(&surroundings);
        use Action::*;

        // Iteratively stable-sort the vector until the ideal plan is found.
        //
        // Plans that involve bombing (and we know are safe) are always best.
        plans.sort_by(
            |&MultiTurnPlan { first_action: a, .. }, &MultiTurnPlan { first_action: b, .. }| match (
                a, b,
            ) {
                (DropBomb | DropBombAndMove(_), Move(_) | StayStill) => Ordering::Less,
                (DropBomb | DropBombAndMove(_), DropBomb | DropBombAndMove(_)) => Ordering::Equal,
                (StayStill | Move(_), StayStill | Move(_)) => Ordering::Equal,
                (Move(_) | StayStill, DropBomb | DropBombAndMove(_)) => Ordering::Greater,
            },
        );
        // Prioritize plans that get us to new areas, unless the current area is hilly
        plans.sort_by(|a, b| {
            match (
                self.boring_tiles.contains(&a.next_position()),
                self.boring_tiles.contains(&b.next_position()),
            ) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                _ => Ordering::Equal,
            }
        });
        // Get us to the hills!
        plans.sort_by(|a, b| match (a.final_tile().0, b.final_tile().0) {
            (Tile::Hill, Tile::Wall | Tile::Floor) => Ordering::Less,
            (Tile::Wall | Tile::Floor, Tile::Hill) => Ordering::Greater,
            _ => Ordering::Equal,
        });
        // As a maximum priority, choose plans that get us new powerups
        plans.sort_by(|a, b| {
            let a_has_powerup = surroundings.iter().any(|(_, obj, _, off)| {
                matches!(obj, Some(Object::PowerUp(_))) && off == &a.next_position()
            });
            let b_has_powerup = surroundings.iter().any(|(_, obj, _, off)| {
                matches!(obj, Some(Object::PowerUp(_))) && off == &b.next_position()
            });
            match (a_has_powerup, b_has_powerup) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Equal,
                (false, false) => Ordering::Equal,
            }
        });

        // Choose the best plan, and if there's none just stand still and await for death :(
        let action = plans
            .get(0)
            .map(|MultiTurnPlan { first_action, .. }| first_action)
            .cloned()
            .unwrap_or(Action::StayStill);

        // Update the visited tiles with the current one, and adjust them for movement. Do not
        // add to boring tiles if it's hilly.
        if surroundings
            .iter()
            .any(|(tile, _, _, off)| off == &TileOffset(0, 0) && tile != &Tile::Hill)
        {
            self.boring_tiles.push(TileOffset(0, 0));
        }
        for tile in self.boring_tiles.iter_mut() {
            let displacement = match action {
                Move(d) => d.extend(-1),
                DropBombAndMove(d) => d.extend(-1),
                _ => TileOffset(0, 0),
            };
            *tile = *tile + displacement;
        }
        action
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

    #[test]
    fn sorting_plans() {
        let mut plans = vec![
            MultiTurnPlan {
                first_action: Action::Move(Direction::North),
                final_surroundings: vec![],
            },
            MultiTurnPlan { first_action: Action::DropBomb, final_surroundings: vec![] },
        ];

        use Action::*;
        plans.sort_by(
            |&MultiTurnPlan { first_action: a, .. }, &MultiTurnPlan { first_action: b, .. }| match (
                a, b,
            ) {
                (DropBomb | DropBombAndMove(_), Move(_) | StayStill) => Ordering::Less,
                (DropBomb | DropBombAndMove(_), DropBomb | DropBombAndMove(_)) => Ordering::Equal,
                (StayStill | Move(_), StayStill | Move(_)) => Ordering::Equal,
                (Move(_) | StayStill, DropBomb | DropBombAndMove(_)) => Ordering::Greater,
            },
        );

        assert_eq!(plans[0].first_action, Action::DropBomb)
    }

    #[test]
    fn bomb_ranges() {
        let surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, TileOffset)> = vec![
            (
                Tile::Floor,
                Some(Object::Bomb { fuse_remaining: Ticks(0), range: 3 }),
                None,
                TileOffset(-2, 1),
            ),
            (Tile::Floor, None, None, TileOffset(-1, 1)),
            (Tile::Floor, None, None, TileOffset(0, 1)),
            (
                Tile::Floor,
                Some(Object::Bomb { fuse_remaining: Ticks(2), range: 3 }),
                None,
                TileOffset(1, 1),
            ),
            (Tile::Floor, None, None, TileOffset(0, 1)),
            (
                Tile::Floor,
                Some(Object::Bomb { fuse_remaining: Ticks(2), range: 3 }),
                None,
                TileOffset(1, 2),
            ),
            (Tile::Wall, None, None, TileOffset(2, 1)),
            (Tile::Floor, Some(Object::Crate), None, TileOffset(3, 1)),
        ];

        let bombs = bombs(&surroundings);
        let empty_tiles = empty_tiles(&surroundings);
        let mut bombs_about_to_explode: Vec<Bomb> =
            bombs.iter().cloned().filter(|(Ticks(t), _, _)| *t == 0).collect();

        // iteratively add all bombs that will be triggered by bombs about to explode.
        iterative_explosions(&mut bombs_about_to_explode, &bombs, &empty_tiles);
        assert_eq!(bombs_about_to_explode.len(), 3);
        assert_eq!(bombs_about_to_explode[0].2, TileOffset(-2, 1));
        assert_eq!(bombs_about_to_explode[1].2, TileOffset(1, 1));
        assert_eq!(bombs_about_to_explode[2].2, TileOffset(1, 2));
    }

    #[test]
    fn sample_turn() {
        // "XX.XX"    X = wall,  P = player
        // ".BP.X"    . = empty, B = bomb
        // "XXXXX"
        // surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, TileOffset)>,
        #[rustfmt::skip]
        let surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, TileOffset)> =
            vec![
                (Tile::Wall, None, None, TileOffset(-2, 1)),
                (Tile::Wall, None, None, TileOffset(-1, 1)),
                (Tile::Floor, None, None, TileOffset(0, 1)),
                (Tile::Wall, None, None, TileOffset(1, 1)),
                (Tile::Wall, None, None, TileOffset(2, 1)),

                (Tile::Floor, None, None, TileOffset(-2, 0)),
                (Tile::Floor, Some(Object::Bomb { fuse_remaining: Ticks(0), range: 3}), None, TileOffset(-1, 0)),
                (Tile::Floor, None, None, TileOffset(0, 0)),
                (Tile::Floor, None, None, TileOffset(1, 0)),
                (Tile::Wall, None, None, TileOffset(2, 0)),
                
                
                (Tile::Wall, None, None, TileOffset(-2, -1)),
                (Tile::Wall, None, None, TileOffset(-1, -1)),
                (Tile::Wall, None, None, TileOffset(0, -1)),
                (Tile::Wall, None, None, TileOffset(1, -1)),
                (Tile::Wall, None, None, TileOffset(2, -1)),
            ];

        let mut player = Bomber { boring_tiles: vec![] };
        let decision = player.act(surroundings);
        assert_eq!(decision, Action::Move(Direction::North));
    }
}
