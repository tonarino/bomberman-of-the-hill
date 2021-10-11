use std::{sync::Arc, thread, time::Duration};
use bevy::prelude::*;

use hero_behaviour::HeroBehaviourPlugin;
use hero_lib::{Action, world::{Direction, Tile, World}};
use labyrinth::Labyrinth;
use rendering::draw_labyrinth;
use wasmtime::{Caller, Engine, Func, Instance, Module, Store};

static WANDERER_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/wanderer.wasm");
static FOOL_WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/debug/fool.wasm");

mod labyrinth;
mod rendering;
mod hero_hotswap;
mod hero_behaviour;

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
