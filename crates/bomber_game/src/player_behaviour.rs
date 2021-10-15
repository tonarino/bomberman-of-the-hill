//! Defines a Bevy plugin that governs spawning and despawning players from .wasm handles,
//! as well as the continuous behaviour of players as they exist in the game world.

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bomber_lib::{wasm_act, wasm_name, Action, LastTurnResult};
use wasmtime::Store;

use crate::{
    error_sink,
    game_map::{self, GameMap, TileLocation, INITIAL_LOCATION},
    player_hotswap::{PlayerHandles, WasmPlayerAsset},
    rendering::{GAME_MAP_Z, TILE_WIDTH_PX},
};

pub struct PlayerBehaviourPlugin;
/// Marks a player
struct Player;
/// Marks the timer used to sequence all player actions (the universal tick)
struct PlayerTimer;
/// Marks the skull sprite used to signal a player death for a few seconds
struct DeathMarker;

impl Plugin for PlayerBehaviourPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .insert_resource(wasmtime::Engine::default())
            .add_system(player_spawn_system.system())
            .add_system(player_positioning_system.system())
            .add_system(player_movement_system.system().chain(error_sink.system()))
            .add_system(death_marker_cleanup_system.system());
    }
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(Timer::from_seconds(1.0, true)).insert(PlayerTimer);
}

/// Ensures the number of active live players matches the `.wasm` files under `assets/players`
/// at all times, by recursively spawning and despawning players.
#[allow(clippy::too_many_arguments)]
fn player_spawn_system(
    mut commands: Commands,
    handles: Res<PlayerHandles>,
    players: Query<(Entity, &Handle<WasmPlayerAsset>), With<Player>>,
    game_map: Res<GameMap>,
    engine: Res<wasmtime::Engine>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WasmPlayerAsset>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Despawn all excess players (if the wasm file was unloaded)
    for (entity, handle) in players.iter() {
        if handles.0.iter().all(|h| h.id != handle.id) {
            commands.entity(entity).despawn_recursive();
        }
    }
    // Spawn all missing players (if the wasm file was just loaded)
    for handle in handles.0.iter() {
        if players.iter().all(|(_, h)| h.id != handle.id) {
            spawn_player(
                handle.clone(),
                &game_map,
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

/// Loads the `.wasm` bytes, JIT compiles them and stores all player-related state
/// in an entity. The import functions binding is done here, which means players effectively
/// get a "callback" into the world to use as they remain alive.
fn spawn_player(
    handle: Handle<WasmPlayerAsset>,
    game_map: &GameMap,
    engine: &wasmtime::Engine,
    asset_server: &AssetServer,
    assets: &Assets<WasmPlayerAsset>,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
) -> Result<(), anyhow::Error> {
    // The Store owns all player-adjacent data internal to the wasm module
    let mut store = Store::new(engine, ());
    let wasm_bytes = assets
        .get(&handle)
        .ok_or_else(|| anyhow!("Wasm asset not found at runtime"))?
        .bytes
        .clone();

    // Here the raw `wasm` is JIT compiled into a stateless module.
    let module = wasmtime::Module::new(engine, wasm_bytes)?;
    // Here the module is bound to a store.
    let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
    let texture_handle = asset_server.load("graphics/player.png");
    // TODO if this fails, the character should immediately be booted out (file deleted) to
    // guarantee stability
    let name = wasm_name(&mut store, &instance)?;
    info!("{} has entered the game!", name);
    commands
        .spawn()
        .insert(Player)
        .insert(instance)
        .insert(store)
        .insert(INITIAL_LOCATION)
        .insert(handle)
        .insert(name)
        .insert_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_translation(
                INITIAL_LOCATION.as_pixels(game_map, GAME_MAP_Z + 1.0),
            ),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            ..Default::default()
        });
    Ok(())
}

/// Continuously updates the player transform to match its abstract location
/// in the game_map.
fn player_positioning_system(
    game_map: Res<GameMap>,
    mut players: Query<(&mut Transform, &TileLocation), With<Player>>,
) {
    for (mut transform, location) in players.iter_mut() {
        transform.translation = location.as_pixels(&game_map, GAME_MAP_Z + 1.0);
    }
}

/// Every universal tick, queries all players for their desired action and applies
/// it. At the moment this only results in movement (or death) but will likely expand
/// into more complex actions.
fn player_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<PlayerTimer>>,
    mut player_query: Query<
        (Entity, &mut TileLocation, &mut wasmtime::Store<()>, &wasmtime::Instance),
        With<Player>,
    >,
    game_map: Res<GameMap>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (entity, mut location, mut store, instance) in player_query.iter_mut() {
            let action = wasm_player_action(&mut store, instance, &location, &game_map);
            apply_action(
                &mut commands,
                &asset_server,
                &mut materials,
                action?,
                &mut location,
                &game_map,
                entity,
            );
        }
    }
    Ok(())
}

/// Applies the action chosen by a player, causing an impact on the world or itself.
fn apply_action(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    action: Action,
    location: &mut TileLocation,
    game_map: &GameMap,
    player_entity: Entity,
) {
    let new_location = match action {
        Action::Move(direction) => (*location + direction).unwrap_or(*location),
        Action::StayStill => *location,
    };

    match game_map.tile(new_location) {
        Some(bomber_lib::world::Tile::Wall) => {
            info!("A player ({:?}) bumps into a wall at {:?}.", player_entity, new_location)
        },
        Some(bomber_lib::world::Tile::EmptyFloor) => {
            info!("A player ({:?}) walks into {:?}", player_entity, new_location);
            *location = new_location;
        },
        Some(bomber_lib::world::Tile::Switch) => {
            info!("A player ({:?}) presses a switch at {:?}", player_entity, new_location)
        },
        Some(bomber_lib::world::Tile::Lava) => {
            info!("A player ({:?}) dissolves in lava at {:?}", player_entity, new_location);
            kill_player(commands, asset_server, materials, player_entity, new_location, game_map);
        },
        None => {
            info!(
                "A player ({:?}) somehow walks into the void at {:?}...",
                player_entity, new_location
            );
            kill_player(commands, asset_server, materials, player_entity, new_location, game_map);
        },
    };
}

/// Despawns a player and leaves a death marker for a few seconds.
fn kill_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    player_entity: Entity,
    new_location: game_map::TileLocation,
    game_map: &GameMap,
) {
    let texture_handle = asset_server.load("graphics/death.png");
    commands.entity(player_entity).despawn_recursive();
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            transform: Transform::from_translation(
                new_location.as_pixels(game_map, GAME_MAP_Z + 1.0),
            ),
            ..Default::default()
        })
        .insert(DeathMarker)
        .insert(Timer::from_seconds(2.0, false));
}

/// Cleans up death markers as their timers expire.
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

/// Executes the `.wasm` export to get the player's decision given its current surroundings.
fn wasm_player_action(
    store: &mut wasmtime::Store<()>,
    instance: &wasmtime::Instance,
    location: &TileLocation,
    game_map: &GameMap,
) -> Result<Action> {
    let last_result = LastTurnResult::StoodStill; // TODO close the LastTurnResult loop.
    let tiles = game_map.tiles_surrounding_location(*location);
    wasm_act(store, instance, tiles, last_result)
}
