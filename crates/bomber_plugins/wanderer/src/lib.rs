use std::convert::TryFrom;

use bomber_lib::{
    self,
    world::{Direction, Tile},
    Action, LastTurnResult, Player,
};
use bomber_macro::wasm_export;

struct Wanderer {
    preferred_direction: Direction,
}

impl Default for Wanderer {
    fn default() -> Self {
        Self { preferred_direction: Direction::North }
    }
}

#[wasm_export]
impl Player for Wanderer {
    fn act(
        &mut self,
        surroundings: Vec<(Tile, bomber_lib::world::Distance)>,
        _last_result: LastTurnResult,
    ) -> Action {
        // A wanderer walks to their preferred direction if it's free.
        // If it isn't, they  walk to the first free tile they inspect.
        let preferred_tile =
            surroundings.iter().find_map(|(t, p)| (*p == self.preferred_direction * 1).then(|| t));
        if matches!(preferred_tile, Some(Tile::EmptyFloor)) {
            Action::Move(Direction::North)
        } else {
            surroundings
                .iter()
                .filter(|(t, p)| p.adjacent() && matches!(t, Tile::EmptyFloor))
                .find_map(|(_, p)| Direction::try_from(*p).map(Action::Move).ok())
                .unwrap_or(Action::StayStill)
        }
    }

    fn name(&self) -> String {
        "Wanderman".into()
    }
}
