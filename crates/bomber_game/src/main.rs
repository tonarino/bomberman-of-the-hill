use bevy::prelude::*;
use player_hotswap::PlayerHotswapPlugin;
use std::{convert::TryFrom, sync::Arc};
use anyhow::Result;

use player_behaviour::PlayerBehaviourPlugin;
use game_map::GameMap;
use rendering::draw_game_map;

mod player_behaviour;
mod player_hotswap;
mod game_map;
mod rendering;

fn main() -> Result<()> {
    let game_map = GameMap::try_from(game_map::DANGEROUS)?;
    App::build()
        .insert_resource(Arc::new(game_map))
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::audio::AudioPlugin>()
        })
        .add_plugin(PlayerBehaviourPlugin)
        .add_plugin(PlayerHotswapPlugin)
        .add_startup_system(setup.system())
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_map: Res<Arc<GameMap>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    draw_game_map(&mut commands, &game_map, &mut materials);
}

// General purpose newtype
pub(crate) struct Wrapper<T>(pub T);
