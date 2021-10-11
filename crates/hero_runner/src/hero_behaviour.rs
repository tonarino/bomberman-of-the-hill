use std::sync::Arc;

use bevy::prelude::*;
use hero_lib::Action;
use wasmtime::{Caller, Engine, Func, Module, Store};

use crate::{FOOL_WASM, labyrinth::{self, INITIAL_LOCATION, Labyrinth}, rendering::{LABYRINTH_Z, TILE_WIDTH_PX}};

pub struct HeroBehaviourPlugin;

struct Hero { location: labyrinth::Location, }

struct HeroTimer;

impl Plugin for HeroBehaviourPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .insert_resource(wasmtime::Engine::default())
            .add_system(hero_positioning_system.system())
            .add_system(hero_movement_system.system());
    }
}

fn setup(
    mut commands: Commands,
    engine: Res<wasmtime::Engine>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(HeroTimer);

    let texture_handle = asset_server.load("graphics/hero.png");
        // This will happen on hotswap. Hardcoded here for now:
        let hero = Hero { location: INITIAL_LOCATION };
        let module = wasmtime::Module::new(&engine, FOOL_WASM).unwrap();
        commands
            .spawn()
            .insert(hero)
            .insert(module)
            .insert_bundle(SpriteBundle {
                material: materials.add(texture_handle.into()),
                sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
                ..Default::default()
            });
}

fn hero_positioning_system(
    labyrinth: Res<Labyrinth>,
    mut hero_query: Query<(&mut Transform, &Hero)>
) {
    for (mut transform, hero) in hero_query.iter_mut() {
        transform.translation = hero.location.as_pixels(&labyrinth, LABYRINTH_Z + 1.0);
    }
}

fn hero_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<HeroTimer>>,
    mut hero_query: Query<(Entity, &mut Hero, &mut wasmtime::Module)>,
    labyrinth: Res<Labyrinth>,
    engine: Res<wasmtime::Engine>,
    mut commands: Commands,
) {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (entity, mut hero, mut module) in hero_query.iter_mut() {
            let action = wasm_hero_action(&engine, &labyrinth, &mut hero, &mut module,);
            apply_action(&mut commands, action, &mut hero, &labyrinth, entity);
        }
    }
}

fn apply_action(commands: &mut Commands, action: Action, hero: &mut Hero, labyrinth: &Labyrinth, hero_entity: Entity) {
    let new_location = match action {
        Action::Move(direction) => (hero.location + direction).unwrap_or(hero.location),
        Action::StayStill => hero.location,
    };

    match labyrinth.tile(new_location) {
        Some(hero_lib::world::Tile::Wall) => println!("The hero bumps into a wall at {:?}.", new_location),
        Some(hero_lib::world::Tile::EmptyFloor) => {
            println!("The hero walks into {:?}", new_location);
            hero.location = new_location;
        },
        Some(hero_lib::world::Tile::Switch) => println!("The hero presses a switch at {:?}", new_location),
        Some(hero_lib::world::Tile::Lava) => {
            println!("The hero dissolves in lava at {:?}", new_location);
            kill_hero(commands, hero_entity, new_location);
        }
        None => {
            println!("The hero somehow walks into the void at {:?}...", new_location);
            kill_hero(commands, hero_entity, new_location);
        }
    };
}

fn kill_hero(commands: &mut Commands, hero_entity: Entity, new_location: labyrinth::Location) {
    commands.entity(hero_entity).despawn_recursive();
}

fn wasm_hero_action(engine: &wasmtime::Engine, labyrinth: &Labyrinth, hero: &mut Hero, module: &mut wasmtime::Module) -> Action {
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
    Action::from(act.call(&mut store, ()).unwrap())
}
