use bevy::prelude::*;
use bomber_lib::world::Tile;

use crate::game_map::{GameMap, Location};

pub const TILE_WIDTH_PX: f32 = 50.0;
pub const GAME_MAP_Z: f32 = 0.0;

impl Location {
    pub fn as_pixels(&self, game_map: &GameMap, z: f32) -> Vec3 {
        let (width, height) = game_map.size();
        let game_map_offset = Vec2::new(
            -(TILE_WIDTH_PX / 2.0) * width as f32,
            -(TILE_WIDTH_PX / 2.0) * height as f32,
        );
        Vec3::new(
            game_map_offset.x + (self.0 as f32) * TILE_WIDTH_PX,
            game_map_offset.y + (self.1 as f32) * TILE_WIDTH_PX,
            z,
        )
    }
}

pub fn draw_game_map(
    commands: &mut Commands,
    game_map: &GameMap,
    materials: &mut Assets<ColorMaterial>,
) {
    let (floor, wall, lava, switch) = (
        materials.add(Color::DARK_GREEN.into()),
        materials.add(Color::BLACK.into()),
        materials.add(Color::RED.into()),
        materials.add(Color::BLUE.into()),
    );
    let (width, height) = game_map.size();
    println!("Game map size is {}, {}", width, height);
    for i in 0..width {
        for j in 0..height {
            let material = match game_map.tile(Location(i, j)) {
                Some(Tile::Wall) => &wall,
                Some(Tile::EmptyFloor) => &floor,
                Some(Tile::Lava) => &lava,
                Some(Tile::Switch) => &switch,
                None => panic!("Expected tile at ({},{})", i, j),
            };

            let game_map_offset = Vec2::new(
                -(TILE_WIDTH_PX / 2.0) * width as f32,
                -(TILE_WIDTH_PX / 2.0) * height as f32,
            );
            let tile_size = Vec2::splat(TILE_WIDTH_PX);
            let tile_position = Vec3::new(
                game_map_offset.x + (i as f32) * TILE_WIDTH_PX,
                game_map_offset.y + (j as f32) * TILE_WIDTH_PX,
                GAME_MAP_Z,
            );

            commands.spawn_bundle(SpriteBundle {
                material: material.clone(),
                sprite: Sprite::new(tile_size),
                transform: Transform::from_translation(tile_position),
                ..Default::default()
            });
        }
    }
}
