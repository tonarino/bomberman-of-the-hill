use std::convert::TryFrom;

use bomber_lib::{
    self,
    world::{Direction, Object, Tile},
    Action, LastTurnResult, Player,
};
use bomber_macro::wasm_export;

struct Wanderer {
    preferred_direction: Direction,
    bomb_ticks: u32,
}

impl Default for Wanderer {
    fn default() -> Self {
        Self { preferred_direction: Direction::North, bomb_ticks: 0 }
    }
}

/// The `Player` implementation block must be decorated with `wasm_export`
/// in order to export the right shims to interface with the bevy `wasm` runtime
#[wasm_export]
impl Player for Wanderer {
    fn act(
        &mut self,
        surroundings: Vec<(Tile, Option<Object>, bomber_lib::world::TileOffset)>,
        _last_result: LastTurnResult,
    ) -> Action {
        // Drops a bomb every once in a while.
        if self.bomb_ticks >= 3 {
            self.bomb_ticks = 0;
            return Action::DropBombAndMove(self.preferred_direction);
        }
        self.bomb_ticks += 1;

        // A wanderer walks to their preferred direction if it's free.
        // If it isn't, they  walk to the first free tile they inspect.
        let preferred_tile = surroundings.iter().find_map(|(t, o, p)| {
            (o.is_none() && (*p == self.preferred_direction.extend(1))).then(|| t)
        });
        if matches!(preferred_tile, Some(Tile::Floor)) {
            Action::Move(Direction::North)
        } else {
            surroundings
                .iter()
                .filter(|(t, o, p)| o.is_none() && p.is_adjacent() && matches!(t, Tile::Floor))
                .find_map(|(_, _, p)| Direction::try_from(*p).map(Action::Move).ok())
                .unwrap_or(Action::StayStill)
        }
    }

    fn name(&self) -> String {
        "Wanderman".into()
    }

    fn team_name() -> String {
        "The Nomads".into()
    }
}
