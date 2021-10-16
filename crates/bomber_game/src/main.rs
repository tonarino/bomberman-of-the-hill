use anyhow::Result;
use bevy::prelude::*;
use player_hotswap::PlayerHotswapPlugin;

use game_map::GameMapPlugin;
use player_behaviour::PlayerBehaviourPlugin;

mod game_map;
mod player_behaviour;
mod player_hotswap;
mod rendering;

fn main() -> Result<()> {
    App::build()
        .add_plugins_with(DefaultPlugins, |group| group.disable::<bevy::audio::AudioPlugin>())
        .add_plugin(GameMapPlugin)
        .add_plugin(PlayerBehaviourPlugin)
        .add_plugin(PlayerHotswapPlugin)
        .add_startup_system(setup.system())
        .run();
    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

#[allow(unused)]
fn log_recoverable_error(In(result): In<Result<()>>) {
    if let Err(e) = result {
        error!("Unhandled error: {}", e);
    }
}

#[allow(unused)]
fn log_unrecoverable_error_and_panic(In(result): In<Result<()>>) {
    if let Err(e) = result {
        error!("Unrecoverable error: {}", e);
        panic!("{}", e);
    }
}

#[allow(unused)]
fn downgrade_error(In(result): In<Result<()>>) {
    if let Err(e) = result {
        info!("Downgraded error: {}", e);
    }
}

// General purpose newtype
pub(crate) struct Wrapper<T>(pub T);
