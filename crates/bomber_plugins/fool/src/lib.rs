use bomber_lib::{
    self,
    world::{Direction, Enemy, Object, Tile},
    Action, Player,
};
use bomber_macro::wasm_export;

#[derive(Default)]
struct Fool;

#[wasm_export]
impl Player for Fool {
    fn act(
        &mut self,
        _surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, bomber_lib::world::TileOffset)>,
    ) -> Action {
        // A fool ignores everything and just walks north!
        Action::Move(Direction::North)
    }

    fn name(&self) -> String {
        "Mr North".into()
    }

    fn team_name() -> String {
        "Northward".into()
    }
}
