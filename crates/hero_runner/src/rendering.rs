use bevy::prelude::*;
use hero_lib::world::Tile::{self, Wall};

use crate::labyrinth::{Labyrinth, Location};

pub const TILE_WIDTH_PX: f32 = 50.0;
pub const LABYRINTH_Z: f32 = 0.0;

impl Location {
    pub fn as_pixels(&self, labyrinth: &Labyrinth, z: f32) -> Vec3 {
        let (width, height) = labyrinth.size();
        let labyrinth_offset = Vec2::new(
            -(TILE_WIDTH_PX / 2.0) * width as f32,
            -(TILE_WIDTH_PX / 2.0) * height as f32,
        );
        Vec3::new(
            labyrinth_offset.x + (self.0 as f32) * TILE_WIDTH_PX,
            labyrinth_offset.y + (self.1 as f32) * TILE_WIDTH_PX,
            z,
        )
    }
}

pub fn draw_labyrinth(
    commands: &mut Commands,
    labyrinth: &Labyrinth,
    materials: &mut Assets<ColorMaterial>,
) {
    let (floor, wall, lava, switch) = (
        materials.add(Color::DARK_GREEN.into()),
        materials.add(Color::BLACK.into()),
        materials.add(Color::RED.into()),
        materials.add(Color::BLUE.into()),
    );
    let (width, height) = labyrinth.size();
    println!("Labyrinth size is {}, {}", width, height);
    for i in 0..width {
        for j in 0..height {
            let material = match labyrinth.tile(Location(i, j)) {
                Some(Tile::Wall) => &wall,
                Some(Tile::EmptyFloor) => &floor,
                Some(Tile::Lava) => &lava,
                Some(Tile::Switch) => &switch,
                None => panic!("Expected tile at ({},{})", i, j),
            };

            let labyrinth_offset = Vec2::new(
                -(TILE_WIDTH_PX / 2.0) * width as f32,
                -(TILE_WIDTH_PX / 2.0) * height as f32,
            );
            let tile_size = Vec2::splat(TILE_WIDTH_PX);
            let tile_position = Vec3::new(
                labyrinth_offset.x + (i as f32) * TILE_WIDTH_PX,
                labyrinth_offset.y + (j as f32) * TILE_WIDTH_PX,
                LABYRINTH_Z,
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
