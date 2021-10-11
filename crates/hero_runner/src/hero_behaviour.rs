use std::sync::Arc;

use bevy::prelude::*;
use hero_lib::Action;
use wasmtime::{Caller, Engine, Func, Module, Store};

use crate::{FOOL_WASM, labyrinth::{self, INITIAL_LOCATION, Labyrinth}};

pub struct HeroBehaviourPlugin;

struct Hero {
    location: labyrinth::Location,
}

struct HeroTimer;

impl Plugin for HeroBehaviourPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system());
        app.insert_resource(wasmtime::Engine::default());
    }
}

fn setup(mut commands: Commands, labyrinth: Res<Arc<Labyrinth>>, engine: Res<wasmtime::Engine>) {
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(HeroTimer);

        // This will happen on hotswap. Hardcoded here for now:
        let hero = Hero { location: INITIAL_LOCATION };
        let module = wasmtime::Module::new(&engine, FOOL_WASM).unwrap();
        commands
            .spawn()
            .insert(hero)
            .insert(module);
}

fn hero_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<HeroTimer>>,
    mut hero_query: Query<(&mut Hero, &mut wasmtime::Module)>,
    labyrinth: Res<Labyrinth>,
    engine: Res<wasmtime::Engine>,
) {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (mut hero, mut module) in hero_query.iter_mut() {
            let action = wasm_hero_action(&engine, &labyrinth, &mut hero, &mut module,);
        }
    }
}

fn wasm_hero_action(engine: &wasmtime::Engine, labyrinth: &Labyrinth, hero: &mut Hero, module: &mut wasmtime::Module) {
    let mut store = Store::new(&engine, (hero, labyrinth));
    let hero_inspect_wasm_import = Func::wrap(&mut store,
        |caller: Caller<'_, (&mut Hero, &Labyrinth)>, direction_raw: u32| -> u32 {
            let (hero, labyrinth) = caller.data();
            labyrinth.inspect_from(hero.location, direction_raw.into()) as u32
        }
    );
    let imports = &[hero_inspect_wasm_import.into()];
    let instance = wasmtime::Instance::new(&mut store, module, imports).unwrap();
    let act = instance.get_typed_func::<(), u32, _>(&mut store, "__act").unwrap();
    let action: Action = Action::from(act.call(&mut store, ()).unwrap());

    match action {
        Action::Move(_) => todo!(),
        Action::StayStill => todo!(),
    }
}
