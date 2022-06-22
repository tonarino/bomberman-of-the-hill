//! Defines a Bevy plugin that governs spawning and despawning players from .wasm handles,
//! as well as the continuous behaviour of players as they exist in the game world.

// Disabling lint for the module because of the ubiquitous Bevy queries.
#![allow(clippy::type_complexity)]

use std::time::Duration;

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bomber_lib::{
    wasm_act, wasm_name, wasm_team_name,
    world::{Direction, Object, Ticks, Tile, TileOffset},
    Action, LastTurnResult,
};
use wasmtime::Store;

use crate::{
    bomb::{KillPlayerEvent, SpawnBombEvent},
    game_map::{GameMap, PlayerSpawner, TileLocation},
    log_recoverable_error, log_unrecoverable_error_and_panic,
    player_hotswap::{PlayerHandle, PlayerHandles, WasmPlayerAsset},
    rendering::{
        PLAYER_HEIGHT_PX, PLAYER_VERTICAL_OFFSET_PX, PLAYER_WIDTH_PX, PLAYER_Z, SKELETON_HEIGHT_PX,
        SKELETON_WIDTH_PX,
    },
    score::Score,
    state::AppState,
    tick::Tick,
    ExternalCrateComponent,
};

pub struct PlayerBehaviourPlugin;

#[derive(Component)]
pub struct PlayerName(pub String);
/// Marks a player
#[derive(Component)]
pub struct Player {
    // The wasm fuel is internally tracked by the store, but it can't be accessed
    // through the `wasmtime` API, so we keep a separate count associated to the player.
    last_turn_total_fuel_consumed: u64,
}
/// Used to mark objects owned by a player entity, such as placed bombs
#[derive(Component)]
pub struct Owner(pub Entity);

/// How far player characters can see their surroundings
const PLAYER_VIEW_TAXICAB_DISTANCE: u32 = 5;

/// Visual representation of a dead player
#[derive(Component)]
struct Skeleton(pub Timer);
/// Visual representation of a banned player
#[derive(Component)]
struct BanSign(pub Timer);
/// It's OK to use seconds rather than ticks for the skeleton and ban sign as it's just a
/// visual representation for fun.
const SKELETON_DURATION: Duration = Duration::from_secs(3);
const BAN_SIGN_DURATION: Duration = Duration::from_secs(3);

const RESPAWN_TIME: Ticks = Ticks(3);
/// Number of allowed WASM instructions per player and per tick. It should be enough to cover non-pathological usage patterns.
/// As a reference, very very basic players like the wanderer and fool spend about 600_000 fuel per turn.
const FUEL_PER_TICK: u64 = 100_000_000;

impl Plugin for PlayerBehaviourPlugin {
    fn build(&self, app: &mut App) {
        let wasm_engine = wasmtime::Engine::new(wasmtime::Config::new().consume_fuel(true))
            .expect("Failed to build wasm engine");
        app.insert_resource(wasm_engine)
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(player_spawn_system)
                    .with_system(
                        player_positioning_system
                            .chain(log_unrecoverable_error_and_panic),
                    )

                    .with_system(player_death_system)
                    .with_system(player_ban_system)
                    .with_system(player_respawn_system)
                    .with_system(skeleton_cleanup_system.chain(log_recoverable_error))
                    .with_system(ban_sign_cleanup_system.chain(log_recoverable_error))
                    .with_system(
                        player_action_system.chain(log_recoverable_error),
                    ),
            )
            // Keep the players on the victory screen as the background.
            .add_system_set(
                SystemSet::on_exit(AppState::VictoryScreen)
                    .with_system(cleanup),
            );
    }
}

/// Ensures the number of active live players matches the `.wasm` files under `assets/players`
/// at all times, by recursively spawning and despawning players.
#[allow(clippy::too_many_arguments)]
fn player_spawn_system(
    mut commands: Commands,
    mut handles: ResMut<PlayerHandles>,
    game_map_query: Query<&GameMap>,
    mut player_query: Query<(Entity, &mut Handle<WasmPlayerAsset>, &TileLocation), With<Player>>,
    spawner_query: Query<&TileLocation, With<PlayerSpawner>>,
    object_query: Query<&TileLocation, With<ExternalCrateComponent<Object>>>,
    engine: Res<wasmtime::Engine>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WasmPlayerAsset>>,
) {
    let game_map = game_map_query.single();
    // Despawn all excess players (if the wasm file was unloaded)
    for (entity, handle, _) in player_query.iter_mut() {
        if handles.0.iter().all(|h| h.inner().id != handle.id) {
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
                    .iter_mut()
                    .all(|(.., player_location)| player_location != spawner_location)
        })
        .collect();

    // Sort them in ascending order of distance to other players
    available_spawn_locations.sort_by_key(|spawner| {
        spawner.taxicab_distance_to_closest(
            player_query.iter_mut().map(|(.., player_location)| player_location).cloned(),
        )
    });
    // Spawn all missing players (if the wasm file was just loaded)
    for (handle, location) in handles
        .0
        .iter_mut()
        .filter(|handle| handle.is_ready_to_spawn())
        .filter(|handle| player_query.iter_mut().all(|(_, h, _)| h.id != handle.inner().id))
        .zip(available_spawn_locations.iter())
    {
        spawn_player(handle, *location, game_map, &engine, &asset_server, &assets, &mut commands)
            .ok();
    }
}

/// Loads the `.wasm` bytes, JIT compiles them and stores all player-related state
/// in an entity. The import functions binding is done here, which means players effectively
/// get a "callback" into the world to use as they remain alive.
#[allow(clippy::too_many_arguments)]
fn spawn_player(
    handle: &mut PlayerHandle,
    location: TileLocation,
    game_map: &GameMap,
    engine: &wasmtime::Engine,
    asset_server: &AssetServer,
    assets: &Assets<WasmPlayerAsset>,
    commands: &mut Commands,
) -> Result<(), anyhow::Error> {
    // The Store owns all player-adjacent data internal to the wasm module
    let mut store = Store::new(engine, ());
    store.add_fuel(FUEL_PER_TICK)?;
    let wasm_bytes = assets
        .get(handle.inner())
        .ok_or_else(|| anyhow!("Wasm asset not found at runtime"))?
        .bytes
        .clone();

    // Here the raw `wasm` is JIT compiled into a stateless module.
    let module = wasmtime::Module::new(engine, wasm_bytes)?;
    // Here the module is bound to a store.
    let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
    let texture_handle = asset_server.load("graphics/Sprites/Bomberman/Front/Bman_F_f00.png");

    let name = if let Ok(name) = wasm_name(&mut store, &instance) {
        name
    } else {
        *handle = PlayerHandle::Misbehaved(handle.inner().clone());
        return Err(anyhow!("Wasm failed to return name, invalidating handle."));
    };
    let name = filter_name(&name);
    let team_name = if let Ok(team_name) = wasm_team_name(&mut store, &instance) {
        team_name
    } else {
        *handle = PlayerHandle::Misbehaved(handle.inner().clone());
        return Err(anyhow!("Wasm failed to return team name, invalidating handle."));
    };

    info!("{} from team {} has entered the game!", name, team_name);
    commands
        .spawn()
        .insert(Player { last_turn_total_fuel_consumed: 0 })
        .insert(ExternalCrateComponent(instance))
        .insert(ExternalCrateComponent(store))
        .insert(location)
        .insert(handle.inner().clone())
        .insert(PlayerName(name.clone()))
        .insert(Score(0))
        .insert_bundle(SpriteBundle {
            texture: texture_handle,
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(PLAYER_Z)
                    + Vec3::new(0.0, PLAYER_VERTICAL_OFFSET_PX, 0.0),
            ),
            sprite: Sprite {
                custom_size: Some(Vec2::new(PLAYER_WIDTH_PX, PLAYER_HEIGHT_PX)),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(move |p| {
            // Text needs to be a child in order to be offset from the player
            // location but still move with the player.
            spawn_player_text(p, asset_server, name);
        });
    Ok(())
}

fn filter_name(name: &str) -> String {
    const MAX_NAME_CHARS: usize = 16;

    // Only take the first line of text, and limit it to 16 chars.
    name.lines()
        .next()
        .map(|line| line.chars().take(MAX_NAME_CHARS).collect())
        .unwrap_or_else(|| "Trickster".to_string())
}

fn spawn_player_text(parent: &mut ChildBuilder, asset_server: &AssetServer, name: String) {
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
    let game_map = game_map_query.single();
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
    mut player_query: Query<(
        Entity,
        &mut TileLocation,
        &mut ExternalCrateComponent<wasmtime::Store<()>>,
        &ExternalCrateComponent<wasmtime::Instance>,
        &PlayerName,
        &mut Player,
        &Handle<WasmPlayerAsset>,
    )>,
    tile_query: Query<
        (&TileLocation, &ExternalCrateComponent<Tile>),
        (Without<Player>, Without<ExternalCrateComponent<Object>>),
    >,
    object_query: Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<Player>, Without<ExternalCrateComponent<Tile>>),
    >,
    mut spawn_bomb_event: EventWriter<SpawnBombEvent>,
    mut ticks: EventReader<Tick>,
    mut handles: ResMut<PlayerHandles>,
) -> Result<()> {
    for _ in ticks.iter().filter(|t| matches!(t, Tick::Player)) {
        for (
            player_entity,
            mut location,
            mut store,
            instance,
            player_name,
            mut player,
            handle_inner,
        ) in player_query.iter_mut()
        {
            let action = match wasm_player_action(
                &mut store,
                instance,
                &location,
                &tile_query,
                &object_query,
            ) {
                Ok(action) => action,
                Err(error) => {
                    error!("Player {} triggered an unrecoverable error ({error:?}). Invalidating handle.", player_name.0);
                    if let Some(handle) =
                        handles.0.iter_mut().find(|handle| handle.inner().id == handle_inner.id)
                    {
                        handle.invalidate();
                    }
                    continue;
                },
            };
            if let Err(e) = apply_action(
                action,
                player_name,
                player_entity,
                &tile_query,
                &object_query,
                &mut spawn_bomb_event,
                &mut location,
            ) {
                // We downgrade this error to informative as the player is allowed
                // to attempt impossible things like walking into a wall (We can later
                // animate these).
                info!("{}", e);
            }

            let total_fuel_consumed =
                store.fuel_consumed().expect("Fuel consumption should be enabled");
            let fuel_consumed_this_turn = total_fuel_consumed
                .checked_sub(player.last_turn_total_fuel_consumed)
                .expect("Invalid fuel count");
            player.last_turn_total_fuel_consumed = total_fuel_consumed;
            info!("{} spent {fuel_consumed_this_turn} fuel this turn.", player_name.0);
            store.add_fuel(fuel_consumed_this_turn)?;
        }
    }
    Ok(())
}

/// If a player "misbehaves" at any point after being spawned (such as by reserving too
/// much memory or spending too much wasm fuel) they will be removed from the game with
/// a visual to represent it, so that the team are made aware there is an issue they
/// need to fix.
fn player_ban_system(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &PlayerName, &Handle<WasmPlayerAsset>), With<Player>>,
    asset_server: Res<AssetServer>,
    mut handles: ResMut<PlayerHandles>,
) {
    for (entity, transform, PlayerName(name), handle_inner) in player_query.iter() {
        if let Some(PlayerHandle::Misbehaved(_)) =
            handles.0.iter_mut().find(|h| h.inner().id == handle_inner.id)
        {
            info!("{name} has been forciby despawned (banned)!");
            commands.entity(entity).despawn_recursive();
            let texture_handle = asset_server.load("graphics/Sprites/Bomberman/Front/Cross.png");
            commands
                .spawn()
                .insert_bundle(SpriteBundle {
                    texture: texture_handle,
                    transform: *transform,
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(SKELETON_WIDTH_PX, SKELETON_HEIGHT_PX)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(BanSign(Timer::new(BAN_SIGN_DURATION, false)));
        }
    }
}

fn player_death_system(
    mut kill_events: EventReader<KillPlayerEvent>,
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &Transform, &PlayerName, &Handle<WasmPlayerAsset>),
        With<Player>,
    >,
    asset_server: Res<AssetServer>,
    mut handles: ResMut<PlayerHandles>,
) {
    for KillPlayerEvent(entity) in kill_events.iter() {
        for (entity, transform, PlayerName(name), handle) in
            player_query.iter_mut().filter(|(e, ..)| e == entity)
        {
            // The handle will be picked up and the player will be automatically respawned with
            // fresh `wasm` state.
            info!("{name} has died!");
            commands.entity(entity).despawn_recursive();
            let texture_handle = asset_server.load("graphics/Sprites/Bomberman/Front/Dead.png");
            commands
                .spawn()
                .insert_bundle(SpriteBundle {
                    texture: texture_handle,
                    transform: *transform,
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(SKELETON_WIDTH_PX, SKELETON_HEIGHT_PX)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Skeleton(Timer::new(SKELETON_DURATION, false)));

            if let Some(handle) = handles.0.iter_mut().find(|h| h.inner().id == handle.id) {
                *handle = PlayerHandle::Respawning(handle.inner().clone(), RESPAWN_TIME);
            }
        }
    }
}

fn player_respawn_system(mut ticks: EventReader<Tick>, mut handles: ResMut<PlayerHandles>) {
    for _ in ticks.iter().filter(|t| matches!(t, Tick::World)) {
        for handle in handles.0.iter_mut() {
            match handle {
                PlayerHandle::ReadyToSpawn(_) => (),
                PlayerHandle::Misbehaved(_) => (),
                PlayerHandle::Respawning(_, Ticks(t)) if *t > 0 => *t -= 1,
                PlayerHandle::Respawning(h, _) => {
                    *handle = PlayerHandle::ReadyToSpawn(h.clone());
                },
            }
        }
    }
}

fn skeleton_cleanup_system(
    mut commands: Commands,
    time: Res<Time>,
    mut skeleton_query: Query<(Entity, &mut Sprite, &mut Skeleton)>,
) -> Result<()> {
    for (entity, mut sprite, mut skeleton) in skeleton_query.iter_mut() {
        let Skeleton(ref mut timer) = *skeleton;
        timer.tick(time.delta());
        // Slowly fade the skeleton
        sprite.color.set_a(timer.percent_left());
        if timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }

    Ok(())
}

fn ban_sign_cleanup_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ban_sign_query: Query<(Entity, &mut Sprite, &mut BanSign)>,
) -> Result<()> {
    for (entity, mut sprite, mut ban_sign) in ban_sign_query.iter_mut() {
        let BanSign(ref mut timer) = *ban_sign;
        timer.tick(time.delta());
        // Slowly fade the ban_sign
        sprite.color.set_a(timer.percent_left());
        if timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }

    Ok(())
}

/// Applies the action chosen by a player, causing an impact on the world or itself.
fn apply_action(
    action: Action,
    player_name: &PlayerName,
    player_entity: Entity,
    tile_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Tile>),
        (Without<Player>, Without<ExternalCrateComponent<Object>>),
    >,
    object_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<Player>, Without<ExternalCrateComponent<Tile>>),
    >,
    spawn_bomb_event: &mut EventWriter<SpawnBombEvent>,
    player_location: &mut TileLocation,
) -> Result<()> {
    match action {
        Action::Move(direction) => {
            move_player(player_name, player_location, direction, tile_query, object_query)
        },
        Action::StayStill => Ok(()),
        Action::DropBomb => {
            spawn_bomb_event
                .send(SpawnBombEvent { location: *player_location, owner: player_entity });
            Ok(())
        },
        Action::DropBombAndMove(direction) => {
            let bomb_location = *player_location;
            move_player(player_name, player_location, direction, tile_query, object_query)?;
            spawn_bomb_event.send(SpawnBombEvent { location: bomb_location, owner: player_entity });
            Ok(())
        },
    }
}

fn move_player(
    player_name: &PlayerName,
    player_location: &mut TileLocation,
    direction: Direction,
    tile_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Tile>),
        (Without<Player>, Without<ExternalCrateComponent<Object>>),
    >,
    object_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<Player>, Without<ExternalCrateComponent<Tile>>),
    >,
) -> Result<()> {
    let PlayerName(player_name) = player_name;

    let target_location = (*player_location + direction)
        .ok_or_else(|| anyhow!("Invalid target location ({})", player_name))?;
    let target_tile = tile_query
        .iter()
        .find_map(|(l, t)| (*l == target_location).then(|| t))
        .ok_or_else(|| anyhow!("No tile at target location ({})", player_name))?;
    let objects_on_target_tile =
        object_query.iter().filter_map(|(l, o)| (*l == target_location).then(|| o)).count();

    match **target_tile {
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
    tile_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Tile>),
        (Without<Player>, Without<ExternalCrateComponent<Object>>),
    >,
    object_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<Player>, Without<ExternalCrateComponent<Tile>>),
    >,
) -> Result<Action> {
    let last_result = LastTurnResult::StoodStill; // TODO close the LastTurnResult loop.
    let player_surroundings: Vec<(Tile, Option<Object>, TileOffset)> = tile_query
        .iter()
        .filter_map(|(location, tile)| {
            let object_on_tile =
                object_query.iter().find_map(|(l, o)| (l == location).then(|| &*o));
            ((*location - *player_location).taxicab_distance() <= PLAYER_VIEW_TAXICAB_DISTANCE)
                .then(|| (**tile, object_on_tile.map(|o| **o), (*location - *player_location)))
        })
        .collect();
    wasm_act(store, instance, player_surroundings, last_result)
}

fn cleanup(player_query: Query<Entity, With<Player>>, mut commands: Commands) {
    for entity in player_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
