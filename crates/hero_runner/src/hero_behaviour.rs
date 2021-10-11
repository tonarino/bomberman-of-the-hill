use std::sync::Arc;

use bevy::prelude::*;
use hero_lib::Action;
use wasmtime::{Caller, Func, Store};

use crate::{
    labyrinth::{self, Labyrinth, INITIAL_LOCATION},
    rendering::{LABYRINTH_Z, TILE_WIDTH_PX},
    FOOL_WASM,
};

pub struct HeroBehaviourPlugin;

struct Hero {
    store: wasmtime::Store<HeroStoreData>,
    instance: wasmtime::Instance,
}

// Contains all state relevant to the wasm hero module
struct HeroStoreData {
    location: labyrinth::Location,
    labyrinth: Arc<Labyrinth>,
}

struct HeroTimer;
struct DeathMarker;

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
    labyrinth: Res<Arc<Labyrinth>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(HeroTimer);

    // This will happen on hotswap. Hardcoded here for now:
    let data = HeroStoreData {
        location: INITIAL_LOCATION,
        labyrinth: labyrinth.clone(),
    };
    let mut store = Store::new(&engine, data);
    let hero_inspect_wasm_import = Func::wrap(
        &mut store,
        |caller: Caller<'_, HeroStoreData>, direction_raw: u32| -> u32 {
            let data = caller.data();
            data.labyrinth
                .inspect_from(data.location, direction_raw.into()) as u32
        },
    );
    let module = wasmtime::Module::new(&engine, FOOL_WASM).unwrap();
    let imports = &[hero_inspect_wasm_import.into()];
    let instance = wasmtime::Instance::new(&mut store, &module, imports).unwrap();
    let hero = Hero { store, instance };

    let texture_handle = asset_server.load("graphics/hero.png");
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
    labyrinth: Res<Arc<Labyrinth>>,
    mut hero_query: Query<(&mut Transform, &Hero)>,
) {
    for (mut transform, hero) in hero_query.iter_mut() {
        transform.translation = hero
            .store
            .data()
            .location
            .as_pixels(&labyrinth, LABYRINTH_Z + 1.0);
    }
}

fn hero_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<HeroTimer>>,
    mut hero_query: Query<(Entity, &mut Hero)>,
    labyrinth: Res<Arc<Labyrinth>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (entity, mut hero) in hero_query.iter_mut() {
            let action = wasm_hero_action(&mut hero);
            apply_action(
                &mut commands,
                &asset_server,
                &mut materials,
                action,
                &mut hero,
                &labyrinth,
                entity,
            );
        }
    }
}

fn apply_action(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    action: Action,
    hero: &mut Hero,
    labyrinth: &Labyrinth,
    hero_entity: Entity,
) {
    let new_location = match action {
        Action::Move(direction) => {
            (hero.store.data().location + direction).unwrap_or(hero.store.data().location)
        }
        Action::StayStill => hero.store.data().location,
    };

    match labyrinth.tile(new_location) {
        Some(hero_lib::world::Tile::Wall) => {
            println!("The hero bumps into a wall at {:?}.", new_location)
        }
        Some(hero_lib::world::Tile::EmptyFloor) => {
            println!("The hero walks into {:?}", new_location);
            hero.store.data_mut().location = new_location;
        }
        Some(hero_lib::world::Tile::Switch) => {
            println!("The hero presses a switch at {:?}", new_location)
        }
        Some(hero_lib::world::Tile::Lava) => {
            println!("The hero dissolves in lava at {:?}", new_location);
            kill_hero(
                commands,
                &asset_server,
                materials,
                hero_entity,
                new_location,
                labyrinth,
            );
        }
        None => {
            println!(
                "The hero somehow walks into the void at {:?}...",
                new_location
            );
            kill_hero(
                commands,
                &asset_server,
                materials,
                hero_entity,
                new_location,
                labyrinth,
            );
        }
    };
}

fn kill_hero(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    hero_entity: Entity,
    new_location: labyrinth::Location,
    labyrinth: &Labyrinth,
) {
    let texture_handle = asset_server.load("graphics/death.png");
    commands.entity(hero_entity).despawn_recursive();
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle.into()),
        sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
        transform: Transform::from_translation(new_location.as_pixels(labyrinth, LABYRINTH_Z + 1.0)),
        ..Default::default()
    }).insert(DeathMarker);
}

fn wasm_hero_action(
    hero: &mut Hero,
) -> Action {
    let act = hero
        .instance
        .get_typed_func::<(), u32, _>(&mut hero.store, "__act")
        .unwrap();
    Action::from(act.call(&mut hero.store, ()).unwrap())
}
