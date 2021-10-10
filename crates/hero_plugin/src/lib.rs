use std::sync::Mutex;

use hero_lib::{self, Action, Hero, world::{Direction, Tile, World}};
use strum::IntoEnumIterator;
use lazy_static::lazy_static;

struct Wanderer;

impl Hero for Wanderer {
    fn spawn() -> Self { Self }

    fn act(&self, world: &impl World) -> Action {
        // A wanderer walks into the first free tile they see.
        let tile_is_free = |d: &Direction| world.inspect(*d) == Tile::EmptyFloor;
        Direction::iter().filter(tile_is_free).next().map(Action::Move).unwrap_or(Action::StayStill)
    }
}

// Abstract these away into a macro
lazy_static! {
    static ref HERO: Mutex<Wanderer> = Mutex::new(Wanderer::spawn());
}

struct _WorldShim;

impl World for _WorldShim {
    fn inspect(&self, direction: Direction) -> Tile {
        unsafe { __inspect(direction as u32).into() }
    }
}

#[no_mangle]
pub fn __act() -> u32 {
    HERO.lock().unwrap().act(&_WorldShim).into()
}

extern { fn __inspect(direction_raw: u32) -> u32; }
