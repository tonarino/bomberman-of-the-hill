use std::ops::{Add, Sub};

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bomber_lib::world::{Direction, Object, Tile, TileOffset};
use rand::Rng;

use crate::{
    bomb::Flame,
    log_unrecoverable_error_and_panic,
    rendering::{GAME_MAP_Z, GAME_OBJECT_Z, TILE_HEIGHT_PX, TILE_WIDTH_PX},
};

/// comfortable for 8 players, many starting crates, open hill in the center.
pub const CRATE_HEAVY_CROSS_ARENA_SMALL: &str =
    include_str!("../assets/maps/crate_heavy_cross_arena_small.txt");

/// Activating this plugin automatically spawns a game map on startup.
pub struct GameMapPlugin;

#[derive(Copy, Clone, Debug)]
pub struct GameMap {
    width: usize,
    height: usize,
}

/// Spawners (represented with a `s` in textual form) designate the tiles in
/// which player characters can appear.
#[derive(Copy, Clone, Debug)]
pub struct PlayerSpawner;

pub struct Textures {
    pub wall: Handle<Texture>,
    pub floor: Handle<Texture>,
    pub hill: Handle<Texture>,
    pub breakable: Handle<Texture>,
}

impl Plugin for GameMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let asset_server =
            app.world().get_resource::<AssetServer>().expect("Failed to retrieve asset server");
        let textures = Textures {
            wall: asset_server.load("graphics/Sprites/Blocks/SolidBlock.png"),
            floor: asset_server.load("graphics/Sprites/Blocks/BackgroundTile.png"),
            hill: asset_server.load("graphics/Sprites/Blocks/BackgroundTileColorShifted.png"),
            breakable: asset_server.load("graphics/Sprites/Blocks/ExplodableBlock.png"),
        };
        app.insert_resource(textures);
        app.add_startup_system(setup.system().chain(log_unrecoverable_error_and_panic.system()))
            .add_system(object_despawn_system.system());
    }
}

fn setup(
    mut commands: Commands,
    textures: Res<Textures>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) -> Result<()> {
    GameMap::spawn_from_text(
        &mut commands,
        CRATE_HEAVY_CROSS_ARENA_SMALL,
        &textures,
        &mut materials,
    )
}

fn object_despawn_system(
    flame_query: Query<&TileLocation, With<Flame>>,
    object_query: Query<(Entity, &TileLocation), With<Object>>,
    mut commands: Commands,
) {
    for (entity, location) in object_query.iter() {
        let on_fire = flame_query.iter().any(|l| *l == *location);
        if on_fire {
            commands.entity(entity).despawn();
        }
    }
}

impl GameMap {
    /// Initializes a game map and spawns all tiles and tile objects from
    /// its textual representation, under a common entity parent.
    pub fn spawn_from_text(
        commands: &mut Commands,
        text: &str,
        textures: &Textures,
        materials: &mut Assets<ColorMaterial>,
    ) -> Result<()> {
        let lines: Vec<&str> = text.lines().rev().collect();
        if lines.windows(2).any(|w| w[0].len() != w[1].len()) {
            return Err(anyhow!("Mismatched row sizes in the game map"));
        }
        if lines.is_empty() || lines[0].is_empty() {
            return Err(anyhow!("Game map must have at least a row and a column"));
        }
        let game_map = GameMap { width: lines[0].len(), height: lines.len() };

        let indexed_characters = lines
            .iter()
            .enumerate()
            .flat_map(|(i, l)| l.chars().enumerate().map(move |(j, c)| (i, j, c)));

        commands.spawn().insert(game_map).insert_bundle(SpriteBundle::default()).with_children(
            |parent| {
                for (i, j, c) in indexed_characters {
                    let location = TileLocation(i, j);
                    Self::spawn_game_elements_from_character(
                        parent, &game_map, location, c, textures, materials,
                    )
                    .expect("Failed to spawn game elements");
                }
            },
        );

        Ok(())
    }

    fn spawn_game_elements_from_character(
        parent: &mut ChildBuilder,
        game_map: &GameMap,
        location: TileLocation,
        character: char,
        textures: &Textures,
        materials: &mut Assets<ColorMaterial>,
    ) -> Result<()> {
        let tile = tile_from_char(character);
        Self::spawn_tile(parent, game_map, tile, location, textures, materials);
        if let Some(object) = object_from_char(character) {
            Self::spawn_object(parent, game_map, object, location, textures, materials)?;
        }
        if let Some(spawner) = spawner_from_char(character) {
            parent.spawn().insert(spawner).insert(location);
        }

        Ok(())
    }

    fn spawn_tile(
        parent: &mut ChildBuilder,
        game_map: &GameMap,
        tile: Tile,
        location: TileLocation,
        textures: &Textures,
        materials: &mut Assets<ColorMaterial>,
    ) {
        let texture = match tile {
            Tile::Wall => &textures.wall,
            Tile::Floor => &textures.floor,
            Tile::Hill => &textures.hill,
        };
        parent.spawn().insert(tile).insert(location).insert_bundle(SpriteBundle {
            material: materials.add(texture.clone().into()),
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(GAME_MAP_Z),
            ),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            ..Default::default()
        });
    }

    fn spawn_object(
        parent: &mut ChildBuilder,
        game_map: &GameMap,
        object: Object,
        location: TileLocation,
        textures: &Textures,
        materials: &mut Assets<ColorMaterial>,
    ) -> Result<()> {
        let texture = match object {
            Object::Crate => &textures.breakable,
            _ => {
                return Err(anyhow!("{:?} can not be spawn during game map creation.", object));
            },
        };
        parent.spawn().insert(object).insert(location).insert_bundle(SpriteBundle {
            material: materials.add(texture.clone().into()),
            transform: Transform::from_translation(
                location.as_world_coordinates(game_map).extend(GAME_OBJECT_Z),
            ),
            sprite: Sprite::new(Vec2::splat(TILE_WIDTH_PX)),
            ..Default::default()
        });

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TileLocation(pub usize, pub usize);

impl TileLocation {
    pub fn as_world_coordinates(&self, game_map: &GameMap) -> Vec2 {
        let width_offset = game_map.width as f32 * TILE_WIDTH_PX / 2.0;
        let height_offset = game_map.height as f32 * TILE_WIDTH_PX / 2.0;
        Vec2::new(
            self.0 as f32 * TILE_WIDTH_PX - width_offset,
            self.1 as f32 * TILE_HEIGHT_PX - height_offset,
        )
    }

    pub fn taxicab_distance_to_closest(
        &self,
        locations: impl Iterator<Item = TileLocation>,
    ) -> u32 {
        locations.fold(u32::MAX, |shortest, location| {
            (*self - location).taxicab_distance().min(shortest)
        })
    }
}

impl Add<Direction> for TileLocation {
    type Output = Option<TileLocation>;

    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::West if self.0 == 0 => None,
            Direction::South if self.1 == 0 => None,
            Direction::West => Some(TileLocation(self.0 - 1, self.1)),
            Direction::North => Some(TileLocation(self.0, self.1 + 1)),
            Direction::East => Some(TileLocation(self.0 + 1, self.1)),
            Direction::South => Some(TileLocation(self.0, self.1 - 1)),
        }
    }
}

impl Add<TileOffset> for TileLocation {
    type Output = TileLocation;

    fn add(self, TileOffset(x, y): TileOffset) -> Self::Output {
        Self((self.0 as i32 + x).max(0) as usize, (self.1 as i32 + y).max(0) as usize)
    }
}

impl Sub<TileLocation> for TileLocation {
    type Output = TileOffset;

    fn sub(self, rhs: TileLocation) -> Self::Output {
        TileOffset(self.0 as i32 - rhs.0 as i32, self.1 as i32 - rhs.1 as i32)
    }
}

// Implemented as a standalone function to bypass the orphan rule, as the tiles
// to convert to are defined in the `hero_lib` crate, which must be kept clean for
// the players
fn tile_from_char(character: char) -> Tile {
    match character {
        '#' => Tile::Wall,
        '~' | 'C' => Tile::Hill,
        _ => Tile::Floor,
    }
}

// Implemented as a standalone function for the same reason as `tile_from_char`
fn object_from_char(character: char) -> Option<Object> {
    match character {
        'c' | 'C' => Some(Object::Crate),
        // Numbers in the map text represent a chance for a crate to spawn.
        p @ '1'..='9' => {
            (p.to_digit(10).unwrap() >= rand::thread_rng().gen_range(1..=10)).then(|| Object::Crate)
        },
        _ => None,
    }
}

// Implemented as a standalone function for the same reason as `tile_from_char`
fn spawner_from_char(character: char) -> Option<PlayerSpawner> {
    (character == 's').then(|| PlayerSpawner)
}
