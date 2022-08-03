//! Defines a Bevy plugin that manages transitions between the game states.

use anyhow::Result;
use bevy::prelude::*;
use std::time::Duration;

use crate::log_unrecoverable_error_and_panic;

pub struct AppStatePlugin;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    /// The main game screen.
    InGame,
    /// Shows the winning players and their points,
    /// as well as a count-down timer until a new game starts.
    VictoryScreen,
}

const GAME_DURATION: Duration = Duration::from_secs(3 * 60);
const VICTORY_SCREEN_DURATION: Duration = Duration::from_secs(20);

#[derive(Component)]
pub struct RoundTimer(pub Timer);

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(app_state_system.chain(log_unrecoverable_error_and_panic))
            .add_state(AppState::InGame);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(RoundTimer(Timer::new(GAME_DURATION, false)));
}

fn app_state_system(
    mut timer_query: Query<(Entity, &mut RoundTimer)>,
    time: Res<Time>,
    mut app_state: ResMut<State<AppState>>,
    mut commands: Commands,
) -> Result<()> {
    let (timer_entity, mut timer) = timer_query.single_mut();

    let RoundTimer(ref mut timer) = *timer;
    if timer.tick(time.delta()).just_finished() {
        let (next_state, next_duration) = match app_state.current() {
            AppState::InGame => (AppState::VictoryScreen, VICTORY_SCREEN_DURATION),
            AppState::VictoryScreen => (AppState::InGame, GAME_DURATION),
        };
        app_state.set(next_state)?;
        commands.entity(timer_entity).despawn();
        commands.spawn().insert(RoundTimer(Timer::new(next_duration, false)));
    }

    Ok(())
}
