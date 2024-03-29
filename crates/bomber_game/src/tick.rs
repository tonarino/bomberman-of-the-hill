use anyhow::Result;
use std::time::Duration;

use crate::{log_unrecoverable_error_and_panic, state::AppState};
use bevy::prelude::*;

/// Helps keep game logic discrete by sending alternative world
/// tick and player tick events. Player ticks sequence all player
/// actions, and world ticks sequence all passive world effects
/// like explosions, crate breakage and points. This ensures
/// there are no race conditions (such as a player moving away from
/// a bomb the same frame it explodes).
pub struct TickPlugin;

#[derive(Component)]
struct TickTimer(pub Timer);
#[derive(Component)]
struct TickCounter(u32);

pub const TICK_PERIOD: Duration = Duration::from_millis(500);
pub const WHOLE_TURN_PERIOD: Duration = Duration::from_millis(1000);

pub enum Tick {
    /// Player actions happen simultaneously during player ticks.
    Player,
    /// World reactions happen simultaneously during world ticks.
    World,
}

impl Plugin for TickPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Tick>()
            .add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup))
            .add_system_set(SystemSet::on_update(AppState::InGame).with_system(tick_system))
            .add_system_set(
                SystemSet::on_exit(AppState::InGame)
                    .with_system(cleanup.chain(log_unrecoverable_error_and_panic)),
            );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(TickTimer(Timer::new(TICK_PERIOD, true))).insert(TickCounter(0));
}

fn tick_system(
    mut timer_query: Query<(&mut TickTimer, &mut TickCounter)>,
    time: Res<Time>,
    mut events: EventWriter<Tick>,
) {
    let (mut timer, mut tick_counter) = timer_query.single_mut();
    let TickTimer(ref mut timer) = *timer;
    if timer.tick(time.delta()).just_finished() {
        let event = if tick_counter.0 % 2 == 0 { Tick::Player } else { Tick::World };
        events.send(event);
        tick_counter.0 += 1;
    }
}

fn cleanup(timer_query: Query<Entity, With<TickTimer>>, mut commands: Commands) -> Result<()> {
    let entity = timer_query.single();
    commands.entity(entity).despawn_recursive();

    Ok(())
}
