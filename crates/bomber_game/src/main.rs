use std::ops::{Deref, DerefMut};

use anyhow::Result;
use bevy::prelude::*;

use bomb::BombPlugin;

use game_map::GameMapPlugin;
use game_ui::GameUiPlugin;
use player_behaviour::PlayerBehaviourPlugin;
use player_hotswap::PlayerHotswapPlugin;
use score::ScorePlugin;
use state::AppStatePlugin;
use tick::TickPlugin;
use victory_screen::VictoryScreenPlugin;

mod bomb;
mod game_map;
mod game_ui;
mod player_behaviour;
mod player_hotswap;
mod rendering;
mod score;
mod state;
mod tick;
mod victory_screen;

// Newtype wrapper to work around orphan rule (for the bevy `Component` trait)
#[derive(Component)]
pub struct ExternalCrateComponent<T>(pub T);

impl<T> Deref for ExternalCrateComponent<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ExternalCrateComponent<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppStatePlugin)
        .add_plugin(GameMapPlugin)
        .add_plugin(TickPlugin)
        .add_plugin(ScorePlugin)
        .add_plugin(PlayerBehaviourPlugin)
        .add_plugin(PlayerHotswapPlugin)
        .add_plugin(BombPlugin)
        .add_plugin(VictoryScreenPlugin)
        .add_plugin(GameUiPlugin)
        .add_startup_system(setup)
        .run();
    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

fn log_recoverable_error(In(result): In<Result<()>>) {
    if let Err(e) = result {
        error!("Unhandled error: {}", e);
    }
}

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
