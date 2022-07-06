use bomber_lib::{
    self,
    world::{Enemy, Object, Tile},
    Action, Player,
};
use bomber_macro::wasm_export;

#[derive(Default)]
struct MemCheater;

#[wasm_export]
impl Player for MemCheater {
    #[allow(clippy::empty_loop)]
    fn act(
        &mut self,
        _surroundings: Vec<(Tile, Option<Object>, Option<Enemy>, bomber_lib::world::TileOffset)>,
    ) -> Action {
        // Look at all this memory!
        let big_vec = vec![0u32; 500_000_000];
        if big_vec.len() > 5000000 {
            Action::StayStill
        } else {
            Action::DropBomb
        }
    }

    fn name(&self) -> String {
        "Big Brain".into()
    }

    fn team_name() -> String {
        "Move things break fast".into()
    }
}
