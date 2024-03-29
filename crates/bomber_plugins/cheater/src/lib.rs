use bomber_lib::{
    self,
    world::{Enemy, Object, Tile},
    Action, Player,
};
use bomber_macro::wasm_export;

#[derive(Default)]
struct Cheater;

#[wasm_export]
impl Player for Cheater {
    #[allow(clippy::empty_loop)]
    fn act(
        &mut self,
        _surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, bomber_lib::world::TileOffset)>,
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
