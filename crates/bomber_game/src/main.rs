use bevy::prelude::*;
use player_hotswap::PlayerHotswapPlugin;
use std::sync::Arc;

use player_behaviour::PlayerBehaviourPlugin;
use labyrinth::Labyrinth;
use rendering::draw_labyrinth;

mod player_behaviour;
mod player_hotswap;
mod labyrinth;
mod rendering;

fn main() {
    let labyrinth = Labyrinth::from(labyrinth::DANGEROUS);
    App::build()
        .insert_resource(Arc::new(labyrinth))
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::audio::AudioPlugin>()
        })
        .add_plugin(PlayerBehaviourPlugin)
        .add_plugin(PlayerHotswapPlugin)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    labyrinth: Res<Arc<Labyrinth>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    draw_labyrinth(&mut commands, &labyrinth, &mut materials);
}

// General purpose newtype
pub(crate) struct Wrapper<T>(pub T);
