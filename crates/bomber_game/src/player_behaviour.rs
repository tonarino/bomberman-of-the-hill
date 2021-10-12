//! Defines a Bevy plugin that governs spawning and despawning players from .wasm handles,
//! as well as the continuous behaviour of players as they exist in the game world.
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bomber_lib::Action;
use wasmtime::{Caller, Func, Store};

use crate::{
    error_sink,
    game_map::{self, GameMap, INITIAL_LOCATION},
    player_hotswap::{PlayerHandles, WasmPlayerAsset},
    rendering::{GAME_MAP_Z, TILE_WIDTH_PX},
};

pub struct PlayerBehaviourPlugin;

/// The `Player` struct is composed of player state (both internal and external
/// to the `wasm` environment, a compiled, live `wasm` instance, and a handle
/// to its associated filesystem asset to regulate spawning and despawning.
struct Player {
    store: wasmtime::Store<PlayerStoreData>,
    instance: wasmtime::Instance,
    handle: Handle<WasmPlayerAsset>,
}

/// Contains all state relevant to the wasm player module. This is data that the
/// player can't access directly from its own context, but that the game needs to track
/// in relation to that player.
struct PlayerStoreData {
    location: game_map::Location,
    game_map: Arc<GameMap>,
}

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
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(PlayerTimer);
}

/// Ensures the number of active live players matches the `.wasm` files under `assets/players`
/// at all times, by recursively spawning and despawning players.
#[allow(clippy::too_many_arguments)]
fn player_spawn_system(
    mut commands: Commands,
    handles: Res<PlayerHandles>,
    players: Query<(Entity, &Player)>,
    game_map: Res<Arc<GameMap>>,
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
        if players
            .iter()
            .all(|(_, player)| player.handle.id != handle.id)
        {
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
    game_map: &Arc<GameMap>,
    engine: &wasmtime::Engine,
    asset_server: &AssetServer,
    assets: &Assets<WasmPlayerAsset>,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
) -> Result<(), anyhow::Error> {
    let data = PlayerStoreData {
        location: INITIAL_LOCATION,
        game_map: game_map.clone(),
    };

    // The Store owns all player-adjacent data, whether it's internal to the wasm module
    // or simply associated to the player (e.g. their position in the map)
    let mut store = Store::new(engine, data);

    // The import bindings allow a player to call back to the game world. Note there is a security
    // implication here; a player may call this function at any time. Currently it only requires
    // shared immutable access through an `Arc`, but if the need arises for mutable access we'll
    // have to worry about metering and avoiding potential deadlocks.
    let player_inspect_wasm_import = Func::wrap(
        &mut store,
        |caller: Caller<'_, PlayerStoreData>, direction_raw: u32| -> u32 {
            // Through the `caller` struct, the `wasm` instance is able to
            // access game state by retrieving a `PlayerStoreData` object.
            let data = caller.data();
            data.game_map
                .inspect_from(data.location, direction_raw.into()) as u32
        },
    );

    let wasm_bytes = assets
        .get(&handle)
        .ok_or_else(|| anyhow!("Wasm asset not found at runtime"))?
        .bytes
        .clone();

    // Here the raw `wasm` is JIT compiled into a stateless module.
    let module = wasmtime::Module::new(engine, wasm_bytes)?;
    let imports = &[player_inspect_wasm_import.into()];
    // Here the module is bound to a store and a set of imports to form a stateful instance.
    let instance = wasmtime::Instance::new(&mut store, &module, imports)?;
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
    game_map: Res<Arc<GameMap>>,
    mut players: Query<(&mut Transform, &Player)>,
) {
    for (mut transform, player) in players.iter_mut() {
        transform.translation = player
            .store
            .data()
            .location
            .as_pixels(&game_map, GAME_MAP_Z + 1.0);
    }
}

/// Every universal tick, queries all players for their desired action and applies
/// it. At the moment this only results in movement (or death) but will likely expand
/// into more complex actions.
fn player_movement_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<PlayerTimer>>,
    mut player_query: Query<(Entity, &mut Player)>,
    game_map: Res<Arc<GameMap>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (entity, mut player) in player_query.iter_mut() {
            let action = wasm_player_action(&mut player);
            apply_action(
                &mut commands,
                &asset_server,
                &mut materials,
                action?,
                &mut player,
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
    player: &mut Player,
    game_map: &Arc<GameMap>,
    player_entity: Entity,
) {
    let new_location = match action {
        Action::Move(direction) => {
            (player.store.data().location + direction).unwrap_or(player.store.data().location)
        }
        Action::StayStill => player.store.data().location,
    };

    match game_map.tile(new_location) {
        Some(bomber_lib::world::Tile::Wall) => {
            info!(
                "A player ({:?}) bumps into a wall at {:?}.",
                player_entity, new_location
            )
        }
        Some(bomber_lib::world::Tile::EmptyFloor) => {
            info!(
                "A player ({:?}) walks into {:?}",
                player_entity, new_location
            );
            player.store.data_mut().location = new_location;
        }
        Some(bomber_lib::world::Tile::Switch) => {
            info!(
                "A player ({:?}) presses a switch at {:?}",
                player_entity, new_location
            )
        }
        Some(bomber_lib::world::Tile::Lava) => {
            info!(
                "A player ({:?}) dissolves in lava at {:?}",
                player_entity, new_location
            );
            kill_player(
                commands,
                asset_server,
                materials,
                player_entity,
                new_location,
                game_map,
            );
        }
        None => {
            info!(
                "A player ({:?}) somehow walks into the void at {:?}...",
                player_entity, new_location
            );
            kill_player(
                commands,
                asset_server,
                materials,
                player_entity,
                new_location,
                game_map,
            );
        }
    };
}

/// Despawns a player and leaves a death marker for a few seconds.
fn kill_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    materials: &mut Assets<ColorMaterial>,
    player_entity: Entity,
    new_location: game_map::Location,
    game_map: &Arc<GameMap>,
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
fn wasm_player_action(player: &mut Player) -> Result<Action> {
    let act = player
        .instance
        .get_typed_func::<(), u32, _>(&mut player.store, "__act")?;
    Ok(Action::from(act.call(&mut player.store, ())?))
}
