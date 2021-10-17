//! Defines a Bevy plugin that governs spawning and despawning players from .wasm handles,
//! as well as the continuous behaviour of players as they exist in the game world.

// Disabling lint for the module because of the ubiquitous Bevy queries.
#![allow(clippy::type_complexity)]

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bomber_lib::{
    wasm_act, wasm_name, wasm_team_name,
    world::{Direction, Object, Tile, TileOffset},
    Action, LastTurnResult,
};
use wasmtime::Store;

use crate::{
    game_map::{GameMap, PlayerSpawner, TileLocation},
    log_recoverable_error, log_unrecoverable_error_and_panic,
    player_hotswap::{PlayerHandles, WasmPlayerAsset},
    rendering::{PLAYER_HEIGHT_PX, PLAYER_VERTICAL_OFFSET_PX, PLAYER_WIDTH_PX, PLAYER_Z},
};

pub struct PlayerBehaviourPlugin;
/// Marks a player
struct Player;
/// Marks the timer used to sequence all player actions (the universal tick)
struct PlayerTimer;
struct PlayerName(String);

/// How far player characters can see their surroundings
const PLAYER_VIEW_TAXICAB_DISTANCE: u32 = 3;

impl Plugin for PlayerBehaviourPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .insert_resource(wasmtime::Engine::default())
            .add_system(player_spawn_system.system())
            .add_system(
                player_positioning_system
                    .system()
                    .chain(log_unrecoverable_error_and_panic.system()),
            )
            .add_system(player_action_system.system().chain(log_recoverable_error.system()));
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
    game_map_query: Query<&GameMap>,
    player_query: Query<(Entity, &Handle<WasmPlayerAsset>, &TileLocation), With<Player>>,
    spawner_query: Query<&TileLocation, With<PlayerSpawner>>,
    object_query: Query<&TileLocation, With<Object>>,
    engine: Res<wasmtime::Engine>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WasmPlayerAsset>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let game_map = game_map_query.single().expect("Game map not found");
    // Despawn all excess players (if the wasm file was unloaded)
    for (entity, handle, _) in player_query.iter() {
        if handles.0.iter().all(|h| h.id != handle.id) {
            commands.entity(entity).despawn_recursive();
        }
    }

    // Retrieve all spawner locations that aren't occupied by an object
    // or another player
    let mut available_spawn_locations: Vec<_> = spawner_query
        .iter()
        .cloned()
        .filter(|spawner_location| {
            object_query.iter().all(|object_location| object_location != spawner_location)
                && player_query
                    .iter()
                    .all(|(.., player_location)| player_location != spawner_location)
        })
        .collect();

    // Sort them in ascending order of distance to other players
    available_spawn_locations.sort_by_key(|spawner| {
        spawner.taxicab_distance_to_closest(
            player_query.iter().map(|(.., player_location)| player_location).cloned(),
        )
    });
    // Spawn all missing players (if the wasm file was just loaded)
    for (handle, location) in handles
        .0
        .iter()
        .filter(|handle| player_query.iter().all(|(_, h, _)| h.id != handle.id))
        .zip(available_spawn_locations.iter())
    {
        spawn_player(
            handle.clone(),
            *location,
            game_map,
            &engine,
            &asset_server,
            &assets,
            &mut commands,
            &mut materials,
        )
        .ok();
    }
}

/// Loads the `.wasm` bytes, JIT compiles them and stores all player-related state
/// in an entity. The import functions binding is done here, which means players effectively
/// get a "callback" into the world to use as they remain alive.
#[allow(clippy::too_many_arguments)]
fn spawn_player(
    handle: Handle<WasmPlayerAsset>,
    location: TileLocation,
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
    let texture_handle = asset_server.load("graphics/Sprites/Bomberman/Front/Bman_F_f00.png");
    // TODO if this fails, the character should immediately be booted out (file deleted) to
    // guarantee stability
    let name = wasm_name(&mut store, &instance)?;
    let team_name = wasm_team_name(&mut store, &instance)?;
    info!("{} from team {} has entered the game!", name, team_name);
    commands
        .spawn()
        .insert(Player)
        .insert(instance)
        .insert(store)
        .insert(location)
        .insert(handle)
        .insert(PlayerName(name.clone()))
        .insert_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(PLAYER_Z)
                    + Vec3::new(0.0, PLAYER_VERTICAL_OFFSET_PX, 0.0),
            ),
            sprite: Sprite::new(Vec2::new(PLAYER_WIDTH_PX, PLAYER_HEIGHT_PX)),
            ..Default::default()
        })
        .with_children(move |p| {
            // Text needs to be a child in order to be offset from the player
            // location but still move with the player.
            spawn_player_text(p, asset_server, name);
        });
    Ok(())
}

fn spawn_player_text(
    parent: &mut bevy::prelude::ChildBuilder<'_, '_>,
    asset_server: &AssetServer,
    name: String,
) {
    parent.spawn().insert_bundle(Text2dBundle {
        text: Text::with_section(
            name,
            TextStyle {
                font: asset_server.load("fonts/space_mono_400.ttf"),
                font_size: 30.0,
                color: Color::WHITE,
            },
            TextAlignment { vertical: VerticalAlign::Center, horizontal: HorizontalAlign::Center },
        ),
        transform: Transform::from_translation(Vec3::new(0.0, 30.0, 0.0)),
        ..Default::default()
    });
}

/// Each frame, matches the player world coordinates to their abstract position
/// in the game world.
fn player_positioning_system(
    game_map_query: Query<&GameMap>,
    mut player_query: Query<(&mut Transform, &TileLocation), With<Player>>,
) -> Result<()> {
    let game_map = game_map_query.single()?;
    for (mut transform, location) in player_query.iter_mut() {
        transform.translation = location.as_world_coordinates(game_map).extend(PLAYER_Z)
            + Vec3::new(0.0, PLAYER_VERTICAL_OFFSET_PX, 0.0);
    }
    Ok(())
}

/// Every universal tick, queries all players for their desired action and applies
/// it. At the moment this only results in movement but will likely expand into more
/// complex actions.
fn player_action_system(
    time: Res<Time>,
    mut timer_query: Query<&mut Timer, With<PlayerTimer>>,
    mut player_query: Query<
        (&mut TileLocation, &mut wasmtime::Store<()>, &wasmtime::Instance, &PlayerName),
        With<Player>,
    >,
    tile_query: Query<(&TileLocation, &Tile), (Without<Player>, Without<Object>)>,
    object_query: Query<(&TileLocation, &Object), (Without<Player>, Without<Tile>)>,
) -> Result<()> {
    let mut timer = timer_query.single_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        for (mut location, mut store, instance, player_name) in player_query.iter_mut() {
            let action =
                wasm_player_action(&mut store, instance, &location, &tile_query, &object_query)?;
            if let Err(e) =
                apply_action(action, player_name, &tile_query, &object_query, &mut location)
            {
                // We downgrade this error to informative as the player is allowed
                // to attempt impossible things like walking into a wall (We can later
                // animate these).
                info!("{}", e);
            }
        }
    }
    Ok(())
}

/// Applies the action chosen by a player, causing an impact on the world or itself.
fn apply_action(
    action: Action,
    player_name: &PlayerName,
    tile_query: &Query<(&TileLocation, &Tile), (Without<Player>, Without<Object>)>,
    object_query: &Query<(&TileLocation, &Object), (Without<Player>, Without<Tile>)>,
    player_location: &mut TileLocation,
) -> Result<()> {
    match action {
        Action::Move(direction) => {
            move_player(player_name, player_location, direction, tile_query, object_query)
        },
        Action::StayStill => {
            info!("{} decides to stay still at {:?}", player_name.0, player_location);
            Ok(())
        },
    }
}

fn move_player(
    player_name: &PlayerName,
    player_location: &mut TileLocation,
    direction: Direction,
    tile_query: &Query<(&TileLocation, &Tile), (Without<Player>, Without<Object>)>,
    object_query: &Query<(&TileLocation, &Object), (Without<Player>, Without<Tile>)>,
) -> Result<()> {
    let player_name = &player_name.0;

    let target_location = (*player_location + direction)
        .ok_or_else(|| anyhow!("Invalid target location ({})", player_name))?;
    let target_tile = tile_query
        .iter()
        .find_map(|(l, t)| (*l == target_location).then(|| t))
        .ok_or_else(|| anyhow!("No tile at target location ({})", player_name))?;
    let objects_on_target_tile =
        object_query.iter().filter_map(|(l, o)| (*l == target_location).then(|| o)).count();

    match target_tile {
        Tile::Floor | Tile::Hill if objects_on_target_tile == 0 => {
            info!("{} moves to {:?}", player_name, target_location);
            *player_location = target_location;
            Ok(())
        },
        _ => Err(anyhow!("Can't move to target tile ({})", player_name)),
    }
}

/// Executes the `.wasm` export to get the player's decision given its current surroundings.
fn wasm_player_action(
    store: &mut wasmtime::Store<()>,
    instance: &wasmtime::Instance,
    player_location: &TileLocation,
    tile_query: &Query<(&TileLocation, &Tile), (Without<Player>, Without<Object>)>,
    object_query: &Query<(&TileLocation, &Object), (Without<Player>, Without<Tile>)>,
) -> Result<Action> {
    let last_result = LastTurnResult::StoodStill; // TODO close the LastTurnResult loop.
    let player_surroundings: Vec<(Tile, Option<Object>, TileOffset)> = tile_query
        .iter()
        .filter_map(|(location, tile)| {
            let object_on_tile = object_query.iter().find_map(|(l, o)| (l == location).then(|| *o));
            ((*location - *player_location).taxicab_distance() <= PLAYER_VIEW_TAXICAB_DISTANCE)
                .then(|| (*tile, object_on_tile, (*location - *player_location)))
        })
        .collect();
    wasm_act(store, instance, player_surroundings, last_result)
}
