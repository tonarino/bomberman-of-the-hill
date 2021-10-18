use anyhow::Result;
use bevy::prelude::*;

use bomb::BombPlugin;
use game_map::GameMapPlugin;
use player_behaviour::PlayerBehaviourPlugin;
use player_hotswap::PlayerHotswapPlugin;
use score::ScorePlugin;
use state::AppStatePlugin;
use tick::TickPlugin;
use victory_screen::VictoryScreenPlugin;

mod bomb;
mod game_map;
mod player_behaviour;
mod player_hotswap;
mod rendering;
mod score;
mod state;
mod tick;
mod victory_screen;

fn main() -> Result<()> {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppStatePlugin)
        .add_plugin(GameMapPlugin)
        .add_plugin(TickPlugin)
        .add_plugin(ScorePlugin)
        .add_plugin(PlayerBehaviourPlugin)
        .add_plugin(PlayerHotswapPlugin)
        .add_plugin(BombPlugin)
        .add_plugin(VictoryScreenPlugin)
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
fn log_and_downgrade_error(In(result): In<Result<()>>) {
    if let Err(e) = result {
        info!("Downgraded error: {}", e);
    }
}
