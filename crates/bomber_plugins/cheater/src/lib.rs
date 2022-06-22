use bomber_lib::{
    self,
    world::{Object, Tile},
    Action, LastTurnResult, Player,
};
use bomber_macro::wasm_export;

#[derive(Default)]
struct Cheater;

#[wasm_export]
impl Player for Cheater {
    #[allow(clippy::empty_loop)]
    fn act(
        &mut self,
        _surroundings: Vec<(Tile, Option<Object>, bomber_lib::world::TileOffset)>,
        _last_result: LastTurnResult,
    ) -> Action {
        // A cheater just tries to break everything.
        loop {}
    }

    fn name(&self) -> String {
        "Hyperlooper".into()
    }

    fn team_name() -> String {
        "Move things break fast".into()
    }
}
