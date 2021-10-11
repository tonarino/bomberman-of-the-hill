use bevy::prelude::*;
use std::sync::Arc;

use hero_behaviour::HeroBehaviourPlugin;
use labyrinth::Labyrinth;
use rendering::draw_labyrinth;

#[allow(unused)]
static WANDERER_WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/debug/wanderer.wasm");
#[allow(unused)]
static FOOL_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/fool.wasm");

mod hero_behaviour;
mod hero_hotswap;
mod labyrinth;
mod rendering;

fn main() {
    let labyrinth = Labyrinth::from(labyrinth::DANGEROUS);
    App::build()
        .insert_resource(Arc::new(labyrinth))
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::audio::AudioPlugin>()
        })
        .add_plugin(HeroBehaviourPlugin)
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
