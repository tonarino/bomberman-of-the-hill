//! Defines a Bevy plugin that governs spawning, exploding and despawning of the bombs and flames.

use bevy::prelude::*;
use bomber_lib::world::{Direction, Object, PowerUp, Ticks, Tile};
use rand::{thread_rng, Rng};

use crate::{
    game_map::{GameMap, TileLocation},
    player_behaviour::{KillPlayerEvent, Owner, Player, PlayerName},
    rendering::{FLAME_Z, GAME_OBJECT_Z, TILE_WIDTH_PX},
    score::Score,
    state::AppState,
    tick::Tick,
    ExternalCrateComponent,
};

// A bomb explodes after this number of ticks since it's placed on the map.
const BOMB_FUSE_LENGTH: Ticks = Ticks(4);
const BASE_BOMB_POWER: u32 = 2;
const CHANCE_OF_POWERUP_ON_CRATE: f32 = 0.2;

pub struct ObjectPlugin;
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
struct BombMarker;
/// Marks the center of an explosion with flames in each direction.
#[derive(Component)]
struct ExplosionMarker;
/// Marks a flame placed on the game map.
#[derive(Component)]
pub struct FlameMarker;
/// Marks a powerup placed on the game map.
#[derive(Component)]
struct PowerUpMarker;

struct Textures {
    bomb: Handle<Image>,
    flame: Handle<Image>,
    bomb_range_power_up: Handle<Image>,
    simultaneous_bombs_power_up: Handle<Image>,
    boots_power_up: Handle<Image>,
}

struct SoundEffects {
    explosion: Handle<AudioSource>,
    drop: Handle<AudioSource>,
}

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        let asset_server =
            app.world.get_resource::<AssetServer>().expect("Failed to retrieve asset server");
        let textures = Textures {
            bomb: asset_server.load("graphics/Sprites/Bomb/Bomb_f01.png"),
            flame: asset_server.load("graphics/Sprites/Flame/Flame_f01.png"),
            bomb_range_power_up: asset_server.load("graphics/Sprites/Powerups/FlamePowerup.png"),
            simultaneous_bombs_power_up: asset_server
                .load("graphics/Sprites/Powerups/BombPowerup.png"),
            boots_power_up: asset_server.load("graphics/Sprites/Powerups/SpeedPowerup.png"),
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
                    .with_system(bomb_spawn_system)
                    .with_system(fuse_remaining_system)
                    .with_system(pick_up_power_up_system)
                    .with_system(bomb_explosion_system)
                    .with_system(objects_on_fire_system)
                    .with_system(explosion_despawn_system),
            )
            .add_system_set(SystemSet::on_exit(AppState::InGame).with_system(cleanup));
    }
}

fn bomb_spawn_system(
    mut spawn_event_reader: EventReader<SpawnBombEvent>,
    game_map_query: Query<&GameMap>,
    bomb_query: Query<&Owner, With<BombMarker>>,
    player_query: Query<&Player>,
    textures: Res<Textures>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut commands: Commands,
) {
    let game_map = game_map_query.single();

    let mut any_bomb_spawned = false;
    for SpawnBombEvent { location, owner } in spawn_event_reader.iter() {
        let player = player_query.get(*owner).expect("Bomb has an invalid owner");
        let maximum_bombs =
            1 + player.power_ups.get(&PowerUp::SimultaneousBombs).copied().unwrap_or_default();
        if bomb_query.iter().filter(|Owner(o)| owner == o).count() < maximum_bombs as usize {
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
        .insert(BombMarker)
        .insert(Owner(owner))
        .insert(ExternalCrateComponent(Object::Bomb { fuse_remaining: BOMB_FUSE_LENGTH }))
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
    mut bomb_query: Query<
        (Entity, &TileLocation, &mut ExternalCrateComponent<Object>),
        With<BombMarker>,
    >,
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

fn bomb_explosion_system(
    mut exploded_bombs: EventReader<BombExplodeEvent>,
    tile_query: Query<(&TileLocation, &ExternalCrateComponent<Tile>)>,
    object_query: Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<BombMarker>, Without<Player>),
    >,
    bomb_query: Query<&Owner, With<BombMarker>>,
    player_query: Query<(&Player, &TileLocation, Entity, &PlayerName, &Score)>,
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
        let owner = bomb_query.get(*bomb).expect("Invalid bomb entity");
        let power = if let Ok((player, ..)) = player_query.get(owner.0) {
            BASE_BOMB_POWER + player.power_ups.get(&PowerUp::BombRange).copied().unwrap_or_default()
        } else {
            BASE_BOMB_POWER
        };
        commands.entity(*bomb).despawn_recursive();
        commands
            .spawn()
            .insert(ExplosionMarker)
            .insert_bundle(SpriteBundle::default())
            .with_children(|parent| {
                spawn_flames(
                    parent,
                    location,
                    &tile_query,
                    &object_query,
                    &player_query,
                    &mut kill_events,
                    power,
                    game_map,
                    &textures,
                );
            });
        any_bomb_exploded = true;
    }

    if any_bomb_exploded {
        audio.play(sound_effects.explosion.clone());
    }
}

fn spawn_flames(
    parent: &mut ChildBuilder,
    bomb_location: &TileLocation,
    tile_query: &Query<(&TileLocation, &ExternalCrateComponent<Tile>)>,
    object_query: &Query<
        (&TileLocation, &ExternalCrateComponent<Object>),
        (Without<BombMarker>, Without<Player>),
    >,
    player_query: &Query<(&Player, &TileLocation, Entity, &PlayerName, &Score)>,
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
            if matches!(tile, Some(ExternalCrateComponent(Tile::Wall))) {
                break;
            }
            spawn_flame(parent, &location, game_map, textures);
            if matches!(object, Some(ExternalCrateComponent(Object::Crate))) {
                // Flame does not extend beyond a crate.
                break;
            }

            if let Some((entity, name, score)) =
                player_query
                    .iter()
                    .find_map(|(_, l, e, n, s)| if *l == location { Some((e, n, s)) } else { None })
            {
                kill_events.send(KillPlayerEvent(entity, name.clone(), *score));
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
    parent.spawn().insert(FlameMarker).insert(*location).insert_bundle(SpriteBundle {
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
    flame_query: Query<&TileLocation, With<FlameMarker>>,
    object_query: Query<(Entity, &TileLocation, &ExternalCrateComponent<Object>)>,
    mut explode_events: EventWriter<BombExplodeEvent>,
    mut commands: Commands,
    game_map_query: Query<&GameMap>,
    textures: Res<Textures>,
) {
    let on_fire = |&(_, location, _): &(_, _, _)| flame_query.iter().any(|l| l == location);
    for (entity, location, object) in object_query.iter().filter(on_fire) {
        match **object {
            Object::Bomb { .. } => {
                explode_events.send(BombExplodeEvent { bomb: entity, location: *location })
            },
            Object::Crate => {
                blow_up_crate(&mut commands, entity, *location, game_map_query.single(), &textures)
            },
            Object::PowerUp(_) => (),
        }
    }
}

fn blow_up_crate(
    commands: &mut Commands,
    entity: Entity,
    location: TileLocation,
    game_map: &GameMap,
    textures: &Textures,
) {
    commands.entity(entity).despawn_recursive();
    let mut rng = thread_rng();
    if rng.gen::<f32>() < CHANCE_OF_POWERUP_ON_CRATE {
        let power_up = match rng.gen_range(0..=2) as u32 {
            0 => PowerUp::BombRange,
            1 => PowerUp::DashBoot,
            2 => PowerUp::SimultaneousBombs,
            _ => unreachable!(),
        };
        spawn_power_up(power_up, commands, location, game_map, textures);
    }
}

fn spawn_power_up(
    power_up: PowerUp,
    commands: &mut Commands,
    location: TileLocation,
    game_map: &GameMap,
    textures: &Textures,
) {
    commands
        .spawn()
        .insert(PowerUpMarker)
        .insert(ExternalCrateComponent(Object::PowerUp(power_up)))
        .insert(location)
        .insert_bundle(SpriteBundle {
            texture: match power_up {
                PowerUp::BombRange => textures.bomb_range_power_up.clone(),
                PowerUp::SimultaneousBombs => textures.simultaneous_bombs_power_up.clone(),
                PowerUp::DashBoot => textures.boots_power_up.clone(),
            },
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(GAME_OBJECT_Z),
            ),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(TILE_WIDTH_PX * 3.0 / 4.0)),
                ..Default::default()
            },
            ..Default::default()
        });
}

fn explosion_despawn_system(
    mut ticks: EventReader<Tick>,
    explosion_query: Query<Entity, With<ExplosionMarker>>,
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

fn pick_up_power_up_system(
    mut ticks: EventReader<Tick>,
    mut player_query: Query<(&mut Player, &TileLocation)>,
    power_up_query: Query<
        (Entity, &ExternalCrateComponent<PowerUp>, &TileLocation),
        Without<Player>,
    >,
    mut commands: Commands,
) {
    for _ in ticks.iter().filter(|t| matches!(t, Tick::World)) {
        for (mut player, player_location) in player_query.iter_mut() {
            if let Some((entity, power_up)) =
                power_up_query.iter().find_map(|(entity, power_up, location)| {
                    (location == player_location).then_some((entity, power_up))
                })
            {
                *player.power_ups.entry(**power_up).or_insert(0) += 1;
                // TODO add some visuals showing the powerup being acquired.
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn cleanup(
    bomb_query: Query<Entity, Or<(With<BombMarker>, With<ExplosionMarker>)>>,
    mut commands: Commands,
) {
    for entity in bomb_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
