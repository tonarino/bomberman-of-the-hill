use std::time::Duration;

use bevy::prelude::*;
use bomber_lib::world;

pub struct AnimationPlugin;
pub struct AnimationTimer(Timer);

const ANIMATION_PERIOD: Duration = Duration::from_millis(200);

#[derive(Component, Debug)]
pub enum AnimationState {
    StandingStill,
    Walking(world::Direction),
}

impl AnimationState {
    fn next_sprite(&self, current_sprite: usize) -> usize {
        let cycle = |direction: world::Direction| match direction {
            world::Direction::West => 15..20usize,
            world::Direction::North => 10..15,
            world::Direction::East => 5..10,
            world::Direction::South => 0..5,
        };

        match self {
            AnimationState::StandingStill => match current_sprite {
                i if cycle(world::Direction::West).contains(&i) => {
                    cycle(world::Direction::West).nth(2).unwrap()
                },
                i if cycle(world::Direction::North).contains(&i) => {
                    cycle(world::Direction::North).nth(2).unwrap()
                },
                i if cycle(world::Direction::East).contains(&i) => {
                    cycle(world::Direction::East).nth(2).unwrap()
                },
                _ => cycle(world::Direction::South).nth(2).unwrap(),
            },
            AnimationState::Walking(direction) => cycle(*direction)
                .skip_while(|i| *i != current_sprite)
                .skip(1)
                .chain(cycle(*direction))
                .next()
                .unwrap(),
        }
    }
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(animate_bomberman_system);
        app.insert_resource(AnimationTimer(Timer::new(ANIMATION_PERIOD, true)));
    }
}

fn animate_bomberman_system(
    mut timer: ResMut<AnimationTimer>,
    time: Res<Time>,
    mut sprite_query: Query<(&AnimationState, &mut TextureAtlasSprite)>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        for (state, mut sprite) in sprite_query.iter_mut() {
            sprite.index = state.next_sprite(sprite.index);
        }
    }
}
