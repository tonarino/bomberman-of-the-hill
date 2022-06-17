//! Defines a Bevy plugin that governs spawning, exploding and despawning of the bombs and flames.

// Disabling lint for the module because of the ubiquitous Bevy queries.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
use bomber_lib::world::{Direction, Object, Ticks, Tile};

use crate::{
    game_map::{GameMap, TileLocation},
    player_behaviour::{Owner, Player},
    rendering::{FLAME_Z, GAME_OBJECT_Z, TILE_WIDTH_PX},
    state::AppState,
    tick::Tick,
    OrphanComponent,
};

// A bomb explodes after this number of ticks since it's placed on the map.
const BOMB_FUSE_LENGTH: Ticks = Ticks(4);
// The initial number of tiles that an explosion reach in each direction.
const INITIAL_BOMB_POWER: u32 = 2;
const MAXIMUM_SIMULTANEOUS_BOMBS: usize = 2;

pub struct BombPlugin;
pub struct KillPlayerEvent(pub Entity);
pub struct BombExplodeEvent {
    pub bomb: Entity,
    pub location: TileLocation,
}

/// Triggers a new bomb to be spawn.
pub struct SpawnBombEvent {
    pub location: TileLocation,
    pub owner: Entity,
}
/// Marks a bomb placed on the game map.
#[derive(Component)]
struct Bomb;
/// Marks the center of an explosion with flames in each direction.
#[derive(Component)]
struct Explosion;
/// Marks a flame placed on the game map.
#[derive(Component)]
pub struct Flame;

struct Textures {
    bomb: Handle<Image>,
    flame: Handle<Image>,
}

struct SoundEffects {
    explosion: Handle<AudioSource>,
    drop: Handle<AudioSource>,
}

impl Plugin for BombPlugin {
    fn build(&self, app: &mut App) {
        let asset_server =
            app.world.get_resource::<AssetServer>().expect("Failed to retrieve asset server");
        let textures = Textures {
            bomb: asset_server.load("graphics/Sprites/Bomb/Bomb_f01.png"),
            flame: asset_server.load("graphics/Sprites/Flame/Flame_f01.png"),
        };
        let sound_effects = SoundEffects {
            explosion: asset_server.load("audio/sound_effects/bomb-explosion.mp3"),
            drop: asset_server.load("audio/sound_effects/bomb-drop.mp3"),
        };
        app.insert_resource(textures)
            .add_event::<KillPlayerEvent>()
            .add_event::<BombExplodeEvent>()
            .insert_resource(sound_effects)
            .add_event::<SpawnBombEvent>()
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(bomb_spawn_system.system())
                    .with_system(fuse_remaining_system.system())
                    .with_system(bomb_explosion_system.system())
                    .with_system(objects_on_fire_system.system())
                    .with_system(explosion_despawn_system.system()),
            )
            .add_system_set(SystemSet::on_exit(AppState::InGame).with_system(cleanup.system()));
    }
}

fn bomb_spawn_system(
    mut spawn_event_reader: EventReader<SpawnBombEvent>,
    game_map_query: Query<&GameMap>,
    bomb_query: Query<&Owner, With<Bomb>>,
    textures: Res<Textures>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut commands: Commands,
) {
    let game_map = game_map_query.single();

    let mut any_bomb_spawned = false;
    for SpawnBombEvent { location, owner } in spawn_event_reader.iter() {
        if bomb_query.iter().filter(|Owner(o)| owner == o).count() < MAXIMUM_SIMULTANEOUS_BOMBS {
            spawn_bomb(location, *owner, game_map, &textures, &mut commands);
            any_bomb_spawned = true;
        } else {
            info!("Failed to spawn bomb: User is at maximum bomb count");
        }
    }

    if any_bomb_spawned {
        audio.play(sound_effects.drop.clone());
    }
}

fn spawn_bomb(
    location: &TileLocation,
    owner: Entity,
    game_map: &GameMap,
    textures: &Textures,
    commands: &mut Commands,
) {
    commands
        .spawn()
        .insert(Bomb)
        .insert(Owner(owner))
        .insert(OrphanComponent(Object::Bomb { fuse_remaining: BOMB_FUSE_LENGTH }))
        .insert(*location)
        .insert_bundle(SpriteBundle {
            texture: textures.bomb.clone(),
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(GAME_OBJECT_Z),
            ),
            sprite: Sprite { custom_size: Some(Vec2::splat(TILE_WIDTH_PX)), ..Default::default() },
            ..Default::default()
        });
}

fn fuse_remaining_system(
    mut ticks: EventReader<Tick>,
    mut bomb_query: Query<(Entity, &TileLocation, &mut OrphanComponent<Object>), With<Bomb>>,
    mut explode_events: EventWriter<BombExplodeEvent>,
) {
    for _ in ticks.iter().filter(|t| matches!(t, Tick::World)) {
        for (bomb, &location, mut object) in bomb_query.iter_mut() {
            let should_explode = match **object {
                Object::Bomb { ref mut fuse_remaining } => {
                    fuse_remaining.0 = fuse_remaining.0.saturating_sub(1);
                    fuse_remaining.0 == 0
                },
                _ => false,
            };

            if should_explode {
                explode_events.send(BombExplodeEvent { bomb, location });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bomb_explosion_system(
    mut exploded_bombs: EventReader<BombExplodeEvent>,
    tile_query: Query<(&TileLocation, &OrphanComponent<Tile>)>,
    object_query: Query<
        (&TileLocation, &OrphanComponent<Object>),
        (Without<Bomb>, Without<Player>),
    >,
    player_query: Query<(&TileLocation, Entity), With<Player>>,
    mut kill_events: EventWriter<KillPlayerEvent>,
    game_map_query: Query<&GameMap>,
    textures: Res<Textures>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut commands: Commands,
) {
    let game_map = game_map_query.single();

    let mut any_bomb_exploded = false;
    for BombExplodeEvent { bomb, location } in exploded_bombs.iter() {
        commands.entity(*bomb).despawn_recursive();
        commands.spawn().insert(Explosion).insert_bundle(SpriteBundle::default()).with_children(
            |parent| {
                spawn_flames(
                    parent,
                    location,
                    &tile_query,
                    &object_query,
                    &player_query,
                    &mut kill_events,
                    INITIAL_BOMB_POWER,
                    game_map,
                    &textures,
                );
            },
        );
        any_bomb_exploded = true;
    }

    if any_bomb_exploded {
        audio.play(sound_effects.explosion.clone());
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_flames(
    parent: &mut ChildBuilder,
    bomb_location: &TileLocation,
    tile_query: &Query<(&TileLocation, &OrphanComponent<Tile>)>,
    object_query: &Query<
        (&TileLocation, &OrphanComponent<Object>),
        (Without<Bomb>, Without<Player>),
    >,
    player_query: &Query<(&TileLocation, Entity), With<Player>>,
    kill_events: &mut EventWriter<KillPlayerEvent>,
    bomb_power: u32,
    game_map: &GameMap,
    textures: &Textures,
) {
    // Spawn a flame at the bomb location.
    spawn_flame(parent, bomb_location, game_map, textures);

    // Spawn flames in each direction.
    for direction in &Direction::all() {
        for reach in 1..=(bomb_power as i32) {
            let location = *bomb_location + direction.extend(reach);
            let tile =
                tile_query.iter().find_map(|(l, t)| if *l == location { Some(t) } else { None });
            let object =
                object_query.iter().find_map(|(l, o)| if *l == location { Some(o) } else { None });
            // Flame can not spawn on the walls.
            if matches!(tile, Some(OrphanComponent(Tile::Wall))) {
                break;
            }
            spawn_flame(parent, &location, game_map, textures);
            if matches!(object, Some(OrphanComponent(Object::Crate))) {
                // Flame does not extend beyond a crate.
                break;
            }

            if let Some(player) =
                player_query.iter().find_map(|(l, e)| if *l == location { Some(e) } else { None })
            {
                kill_events.send(KillPlayerEvent(player));
            }
        }
    }
}

fn spawn_flame(
    parent: &mut ChildBuilder,
    location: &TileLocation,
    game_map: &GameMap,
    textures: &Textures,
) {
    parent.spawn().insert(Flame).insert(*location).insert_bundle(SpriteBundle {
        texture: textures.flame.clone(),
        transform: Transform::from_translation(
            location.as_world_coordinates(game_map).extend(FLAME_Z),
        ),
        sprite: Sprite { custom_size: Some(Vec2::splat(TILE_WIDTH_PX)), ..Default::default() },
        ..Default::default()
    });
}

/// Handle objects being blasted by bomb's explosion.
fn objects_on_fire_system(
    flame_query: Query<&TileLocation, With<Flame>>,
    object_query: Query<(Entity, &TileLocation, &OrphanComponent<Object>)>,
    mut explode_events: EventWriter<BombExplodeEvent>,
    mut commands: Commands,
) {
    let on_fire = |&(_, location, _): &(_, _, _)| flame_query.iter().any(|l| l == location);
    for (entity, location, object) in object_query.iter().filter(on_fire) {
        match **object {
            Object::Bomb { .. } => {
                explode_events.send(BombExplodeEvent { bomb: entity, location: *location })
            },
            Object::Crate => commands.entity(entity).despawn_recursive(),
        }
    }
}

fn explosion_despawn_system(
    mut ticks: EventReader<Tick>,
    explosion_query: Query<Entity, With<Explosion>>,
    mut commands: Commands,
) {
    // We despawn explosions during player ticks as they're just a visual
    // indication; the damage has already been done when spawning the flames.
    for _ in ticks.iter().filter(|t| matches!(t, Tick::Player)) {
        for entity in explosion_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn cleanup(bomb_query: Query<Entity, Or<(With<Bomb>, With<Explosion>)>>, mut commands: Commands) {
    for entity in bomb_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
