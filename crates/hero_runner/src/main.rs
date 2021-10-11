use std::{thread, time::Duration};
use bevy::prelude::*;

use hero_lib::{Action, world::{Direction, Tile, World}};
use wasmtime::{Caller, Engine, Func, Instance, Module, Store};

static WANDERER_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/wanderer.wasm");
static FOOL_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/fool.wasm");

mod labyrinth;

struct SolidWorld {
    tile: Tile // All tiles are the same in a solid world!
}

impl World for SolidWorld {
    fn inspect(&self, _: Direction) -> Tile {
        self.tile
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system());
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    let (floor, walls, lava, switch) = (
        materials.add(Color::BEIGE.into()),
        materials.add(Color::BLACK.into()),
        materials.add(Color::RED.into()),
        materials.add(Color::BLUE.into()),
    );
}

//fn main() {
//    let empty_world = SolidWorld { tile: Tile::EmptyFloor };
//
//    let engine = Engine::default();
//    let mut store = Store::new(&engine, empty_world );
//
//    let host_world_inspect = Func::wrap(&mut store,
//        |caller: Caller<'_, SolidWorld>, direction_raw: u32| -> u32 {
//            let world = caller.data();
//            world.inspect(direction_raw.into()) as u32
//        }
//    );
//
//    let wanderer = Module::new(&engine, WANDERER_WASM).unwrap();
//    let fool = Module::new(&engine, FOOL_WASM).unwrap();
//    let wanderer_imports = &[host_world_inspect.into()][0..wanderer.imports().len().min(1)];
//    let wanderer_instance = Instance::new(&mut store, &wanderer, wanderer_imports).unwrap();
//    let fool_imports = &[host_world_inspect.into()][0..fool.imports().len().min(1)];
//    let fool_instance = Instance::new(&mut store, &fool, fool_imports).unwrap();
//    let wanderer_act = wanderer_instance.get_typed_func::<(), u32, _>(&mut store, "__act").unwrap();
//    let fool_act = fool_instance.get_typed_func::<(), u32, _>(&mut store, "__act").unwrap();
//
//    loop {
//        println!("The wanderer's decision is: {:?}", Action::from(wanderer_act.call(&mut store, ()).unwrap()));
//        println!("The fool's decision is: {:?}", Action::from(fool_act.call(&mut store, ()).unwrap()));
//        thread::sleep(Duration::from_secs(1));
//    }
//}
//
