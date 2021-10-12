use std::sync::Arc;

use anyhow::anyhow;
use bevy::prelude::*;
use bomber_lib::Action;
use wasmtime::{Caller, Func, Store};

use crate::{
    player_hotswap::{PlayerHandles, WasmPlayerAsset},
    labyrinth::{self, Labyrinth, INITIAL_LOCATION},
    rendering::{LABYRINTH_Z, TILE_WIDTH_PX},
};

pub struct PlayerBehaviourPlugin;

struct Player {
    store: wasmtime::Store<PlayerStoreData>,
    instance: wasmtime::Instance,
    handle: Handle<WasmPlayerAsset>,
}

// Contains all state relevant to the wasm player module
struct PlayerStoreData {
    location: labyrinth::Location,
    labyrinth: Arc<Labyrinth>,
}

struct PlayerTimer;
struct DeathMarker;

impl Plugin for PlayerBehaviourPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .insert_resource(wasmtime::Engine::default())
            .add_system(player_spawn_system.system())
            .add_system(player_positioning_system.system())
            .add_system(player_movement_system.system())
            .add_system(death_marker_cleanup_system.system());
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(PlayerTimer);
}

fn player_spawn_system(
    mut commands: Commands,
    handles: Res<PlayerHandles>,
    players: Query<(Entity, &Player)>,
    labyrinth: Res<Arc<Labyrinth>>,
    engine: Res<wasmtime::Engine>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WasmPlayerAsset>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Despawn all excess players (if the wasm file was unloaded)
    for (entity, player) in players.iter() {
        if handles.0.iter().all(|handle| handle.id != player.handle.id) {
            commands.entity(entity).despawn_recursive();
        }
    }
    // Spawn all missing players (if the wasm file was just loaded)
    for handle in handles.0.iter() {
        if players.iter().all(|(_, player)| player.handle.id != handle.id) {
            spawn_player(
                handle.clone(),
                &labyrinth,
                &engine,
                &asset_server,
                &assets,
                &mut commands,
                &mut materials,
            )
            .ok();
        }
    }
}

fn spawn_player(
    handle: Handle<WasmPlayerAsset>,
    labyrinth: &Arc<Labyrinth>,
    engine: &wasmtime::Engine,
    asset_server: &AssetServer,
    assets: &Assets<WasmPlayerAsset>,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
) -> Result<(), anyhow::Error> {
    let data = PlayerStoreData {
        location: INITIAL_LOCATION,
        labyrinth: labyrinth.clone(),
    };
    let mut store = Store::new(&engine, data);
    let player_inspect_wasm_import = Func::wrap(
        &mut store,
        |caller: Caller<'_, PlayerStoreData>, direction_raw: u32| -> u32 {
            let data = caller.data();
            data.labyrinth
                .inspect_from(data.location, direction_raw.into()) as u32
        },
    );

    let wasm_bytes = assets
        .get(&handle)
        .ok_or(anyhow!("Wasm asset not found at runtime"))?
        .bytes
        .clone();

    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let imports = &[player_inspect_wasm_import.into()];
    let instance = wasmtime::Instance::new(&mut store, &module, imports).unwrap();
    let player = Player {
        store,
        instance,
        handle,
    };
    let texture_handle = asset_server.load("graphics/player.png");
    commands
        .spawn()
        .insert(player)
        .insert(module)
        .insert_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_translation(
                INITIAL_LOCATION.as_pixels(labyrinth, LABYRINTH_Z + 1.0),
            ),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            ..Default::default()
        });
    Ok(())
}

fn player_positioning_system(
    labyrinth: Res<Arc<Labyrinth>>,
    mut players: Query<(&mut Transform, &Player)>,
) {
    for (mut transform, player) in players.iter_mut() {
        transform.translation = player
            .store
            .data()
            .location
            .as_pixels(&labyrinth, LABYRINTH_Z + 1.0);
    }
}

fn player_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<PlayerTimer>>,
    mut player_query: Query<(Entity, &mut Player)>,
    labyrinth: Res<Arc<Labyrinth>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (entity, mut player) in player_query.iter_mut() {
            let action = wasm_player_action(&mut player);
            apply_action(
                &mut commands,
                &asset_server,
                &mut materials,
                action,
                &mut player,
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
    player: &mut Player,
    labyrinth: &Arc<Labyrinth>,
    player_entity: Entity,
) {
    let new_location = match action {
        Action::Move(direction) => {
            (player.store.data().location + direction).unwrap_or(player.store.data().location)
        }
        Action::StayStill => player.store.data().location,
    };

    match labyrinth.tile(new_location) {
        Some(bomber_lib::world::Tile::Wall) => {
            println!("The player bumps into a wall at {:?}.", new_location)
        }
        Some(bomber_lib::world::Tile::EmptyFloor) => {
            println!("The player walks into {:?}", new_location);
            player.store.data_mut().location = new_location;
        }
        Some(bomber_lib::world::Tile::Switch) => {
            println!("The player presses a switch at {:?}", new_location)
        }
        Some(bomber_lib::world::Tile::Lava) => {
            println!("The player dissolves in lava at {:?}", new_location);
            kill_player(
                commands,
                &asset_server,
                materials,
                player_entity,
                new_location,
                labyrinth,
            );
        }
        None => {
            println!(
                "The player somehow walks into the void at {:?}...",
                new_location
            );
            kill_player(
                commands,
                &asset_server,
                materials,
                player_entity,
                new_location,
                labyrinth,
            );
        }
    };
}

fn kill_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    player_entity: Entity,
    new_location: labyrinth::Location,
    labyrinth: &Arc<Labyrinth>,
) {
    let texture_handle = asset_server.load("graphics/death.png");
    commands.entity(player_entity).despawn_recursive();
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            transform: Transform::from_translation(
                new_location.as_pixels(labyrinth, LABYRINTH_Z + 1.0),
            ),
            ..Default::default()
        })
        .insert(DeathMarker)
        .insert(Timer::from_seconds(2.0, false));
}

fn death_marker_cleanup_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Timer), With<DeathMarker>>,
    time: Res<Time>,
) {
    for (entity, mut timer) in query.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn wasm_player_action(player: &mut Player) -> Action {
    let act = player
        .instance
        .get_typed_func::<(), u32, _>(&mut player.store, "__act")
        .unwrap();
    Action::from(act.call(&mut player.store, ()).unwrap())
}
