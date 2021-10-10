use hero_lib::{Action, world::{Direction, Tile, World}};
use wasmtime::{Caller, Engine, Func, Instance, Module, Store};

static WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/hero_plugin.wasm");

struct SolidWorld {
    tile: Tile // All tiles are the same in a solid world!
}

impl World for SolidWorld {
    fn inspect(&self, _: Direction) -> Tile {
        self.tile
    }
}

fn main() {
    let empty_world = SolidWorld { tile: Tile::EmptyFloor };

    let engine = Engine::default();
    let mut store = Store::new(&engine, empty_world );

    let host_world_inspect = Func::wrap(&mut store,
        |caller: Caller<'_, SolidWorld>, direction_raw: u32| -> u32 {
            let world = caller.data();
            world.inspect(direction_raw.into()) as u32
        }
    );

    let module = Module::new(&engine, WASM).unwrap();
    let instance = Instance::new(&mut store, &module, &[host_world_inspect.into()]).unwrap();
    let act = instance.get_typed_func::<(), u32, _>(&mut store, "__act").unwrap();
    let action: Action = act.call(&mut store, ()).unwrap().into();
    println!("The hero's decision is: {:?}", action);
}

