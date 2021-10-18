//! Defines a Bevy plugin that governs spawning, exploding and despawning of the bombs and flames.

// Disabling lint for the module because of the ubiquitous Bevy queries.
#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use bomber_lib::world::{Direction, Object, Tile};

use crate::{
    game_map::{GameMap, TileLocation},
    rendering::{FLAME_Z, GAME_OBJECT_Z, TILE_WIDTH_PX},
    tick::Tick,
};

// A bomb explodes after this number of ticks since it's placed on the map.
const BOMB_FUSE_LENGTH: u32 = 3;
// The initial number of tiles that an explosion reach in each direction.
const INITIAL_BOMB_POWER: u32 = 2;

pub struct BombPlugin;

/// Triggers a new bomb to be spawn.
pub struct SpawnBombEvent(pub TileLocation);
/// Marks a bomb placed on the game map.
struct Bomb;
/// Marks the center of an explosion with flames in each direction.
struct Explosion;
/// Marks a flame placed on the game map.
pub struct Flame;

struct Textures {
    bomb: Handle<Texture>,
    flame: Handle<Texture>,
}

struct SoundEffects {
    explosion: Handle<AudioSource>,
    drop: Handle<AudioSource>,
}

impl Plugin for BombPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let asset_server =
            app.world().get_resource::<AssetServer>().expect("Failed to retrieve asset server");
        let textures = Textures {
            bomb: asset_server.load("graphics/Sprites/Bomb/Bomb_f01.png"),
            flame: asset_server.load("graphics/Sprites/Flame/Flame_f01.png"),
        };
        let sound_effects = SoundEffects {
            explosion: asset_server.load("audio/sound_effects/bomb-explosion.mp3"),
            drop: asset_server.load("audio/sound_effects/bomb-drop.mp3"),
        };
        app.insert_resource(textures)
            .insert_resource(sound_effects)
            .add_event::<SpawnBombEvent>()
            .add_system(bomb_spawn_system.system())
            .add_system(bomb_explosion_system.system())
            .add_system(explosion_despawn_system.system());
    }
}

fn bomb_spawn_system(
    mut spawn_event_reader: EventReader<SpawnBombEvent>,
    game_map_query: Query<&GameMap>,
    textures: Res<Textures>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let game_map = game_map_query.single().expect("Failed to retrive game map");

    for SpawnBombEvent(location) in spawn_event_reader.iter() {
        spawn_bomb(
            location,
            game_map,
            &textures,
            &audio,
            &sound_effects,
            &mut materials,
            &mut commands,
        );
    }
}

fn spawn_bomb(
    location: &TileLocation,
    game_map: &GameMap,
    textures: &Textures,
    audio: &Audio,
    sound_effects: &SoundEffects,
    materials: &mut Assets<ColorMaterial>,
    commands: &mut Commands,
) {
    commands
        .spawn()
        .insert(Bomb)
        .insert(Object::Bomb { fuse_remaining: BOMB_FUSE_LENGTH })
        .insert(*location)
        .insert_bundle(SpriteBundle {
            material: materials.add(textures.bomb.clone().into()),
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(GAME_OBJECT_Z),
            ),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            ..Default::default()
        });

    // TODO(ryo): It should play only once even if multiple bombs are spawn at the current tick.
    audio.play(sound_effects.drop.clone());
}

#[allow(clippy::too_many_arguments)]
fn bomb_explosion_system(
    mut ticks: EventReader<Tick>,
    mut bomb_query: Query<(Entity, &TileLocation, &mut Object), With<Bomb>>,
    tile_query: Query<(&TileLocation, &Tile)>,
    object_query: Query<(&TileLocation, &Object), Without<Bomb>>,
    game_map_query: Query<&GameMap>,
    textures: Res<Textures>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let game_map = game_map_query.single().expect("Failed to retrieve game map");

    for _ in ticks.iter().filter(|t| matches!(t, Tick::World)) {
        let mut any_bomb_exploded = false;
        for (entity, location, mut object) in bomb_query.iter_mut() {
            let should_explode = match *object {
                Object::Bomb { fuse_remaining: ref mut fuse_length } => {
                    *fuse_length = fuse_length.saturating_sub(1);
                    *fuse_length == 0
                },
                _ => false,
            };

            if should_explode {
                commands.entity(entity).despawn();
                commands
                    .spawn()
                    .insert(Explosion)
                    .insert_bundle(SpriteBundle::default())
                    .with_children(|parent| {
                        spawn_flames(
                            parent,
                            location,
                            &tile_query,
                            &object_query,
                            INITIAL_BOMB_POWER,
                            game_map,
                            &textures,
                            &mut materials,
                        );
                    });
                any_bomb_exploded = true;
            }
        }

        if any_bomb_exploded {
            audio.play(sound_effects.explosion.clone());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_flames(
    parent: &mut ChildBuilder,
    bomb_location: &TileLocation,
    tile_query: &Query<(&TileLocation, &Tile)>,
    object_query: &Query<(&TileLocation, &Object), Without<Bomb>>,
    bomb_power: u32,
    game_map: &GameMap,
    textures: &Textures,
    materials: &mut Assets<ColorMaterial>,
) {
    // Spawn a flame at the bomb location.
    spawn_flame(parent, bomb_location, game_map, textures, materials);

    // Spawn flames in each direction.
    for direction in &Direction::all() {
        for reach in 1..=(bomb_power as i32) {
            let location = *bomb_location + direction.extend(reach);
            let tile =
                tile_query.iter().find_map(|(l, t)| if *l == location { Some(t) } else { None });
            let object =
                object_query.iter().find_map(|(l, o)| if *l == location { Some(o) } else { None });
            // Flame can not spawn on the walls.
            if matches!(tile, Some(Tile::Wall)) {
                break;
            }
            spawn_flame(parent, &location, game_map, textures, materials);
            if matches!(object, Some(Object::Crate)) {
                // Flame does not extend beyond a crate.
                break;
            }
        }
    }
}

fn spawn_flame(
    parent: &mut ChildBuilder,
    location: &TileLocation,
    game_map: &GameMap,
    textures: &Textures,
    materials: &mut Assets<ColorMaterial>,
) {
    parent.spawn().insert(Flame).insert(*location).insert_bundle(SpriteBundle {
        material: materials.add(textures.flame.clone().into()),
        transform: Transform::from_translation(
            location.as_world_coordinates(game_map).extend(FLAME_Z),
        ),
        sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
        ..Default::default()
    });
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
