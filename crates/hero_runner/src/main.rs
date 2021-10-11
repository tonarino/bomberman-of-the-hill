use std::{sync::Arc, thread, time::Duration};
use bevy::prelude::*;

use hero_lib::{Action, world::{Direction, Tile, World}};
use labyrinth::Labyrinth;
use rendering::draw_labyrinth;
use wasmtime::{Caller, Engine, Func, Instance, Module, Store};

static WANDERER_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/wanderer.wasm");
static FOOL_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/fool.wasm");

mod labyrinth;
mod rendering;
mod hero_hotswap;
mod hero_behaviour;

//struct SolidWorld {
//    tile: Tile // All tiles are the same in a solid world!
//}
//
//impl World for SolidWorld {
//    fn inspect(&self, _: Direction) -> Tile {
//        self.tile
//    }
//}

fn main() {
    App::build()
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::audio::AudioPlugin>()
        })
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    let labyrinth = Labyrinth::from(labyrinth::DANGEROUS);
    draw_labyrinth(&mut commands, &labyrinth, &mut materials);
    commands.insert_resource(labyrinth);
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
