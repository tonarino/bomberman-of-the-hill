use std::time::Duration;

use bevy::prelude::*;
use bomber_lib::world;

use crate::{state::AppState, tick::WHOLE_TURN_PERIOD};

pub struct AnimationPlugin;
pub struct AnimationTimer(Timer);

fn animation_period() -> Duration {
    // 8 steps on the animation cycle
    WHOLE_TURN_PERIOD / 8
}

#[derive(Component, Debug)]
pub enum AnimationState {
    StandingStill,
    Walking(world::Direction, usize),
}

impl AnimationState {
    fn next_sprite(&mut self) -> usize {
        let cycle = |direction: world::Direction| match direction {
            world::Direction::West => [17, 18, 19, 18, 17, 16, 15, 16],
            world::Direction::North => [12, 13, 14, 13, 12, 11, 10, 11],
            world::Direction::East => [7, 8, 9, 8, 7, 6, 5, 6],
            world::Direction::South => [2, 3, 4, 3, 2, 1, 0, 1],
        };

        match self {
            AnimationState::StandingStill => cycle(world::Direction::South)[0],
            AnimationState::Walking(direction, current_cycle_index) => {
                let cycle = cycle(*direction);
                *current_cycle_index = (*current_cycle_index + 1) % cycle.len();
                cycle[*current_cycle_index]
            },
        }
    }
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::InGame).with_system(animate_bomberman_system),
        );
        app.insert_resource(AnimationTimer(Timer::new(animation_period(), true)));
    }
}

fn animate_bomberman_system(
    mut timer: ResMut<AnimationTimer>,
    time: Res<Time>,
    mut sprite_query: Query<(&mut AnimationState, &mut TextureAtlasSprite)>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        for (mut state, mut sprite) in sprite_query.iter_mut() {
            sprite.index = state.next_sprite();
        }
    }
}
