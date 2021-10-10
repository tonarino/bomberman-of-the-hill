use hero_lib::{Action, world::{Direction, Tile, World}};
use wasmtime::{Caller, Engine, Func, Instance, Module, Store};

static WANDERER_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/wanderer.wasm");
static FOOL_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/fool.wasm");

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

    let module = Module::new(&engine, WANDERER_WASM).unwrap();
    let imports = &[host_world_inspect.into()][0..module.imports().len().min(1)];
    let instance = Instance::new(&mut store, &module, imports).unwrap();
    let act = instance.get_typed_func::<(), u32, _>(&mut store, "__act").unwrap();
    println!("The hero's decision is: {:?}", Action::from(act.call(&mut store, ()).unwrap()));
    println!("The hero's decision is: {:?}", Action::from(act.call(&mut store, ()).unwrap()));
    println!("The hero's decision is: {:?}", Action::from(act.call(&mut store, ()).unwrap()));
}

