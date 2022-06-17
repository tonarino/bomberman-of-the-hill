use bevy::prelude::*;
use bomber_lib::world::Tile;

use crate::{game_map::TileLocation, player_behaviour::Player, tick::Tick, OrphanComponent};

pub struct ScorePlugin;
#[derive(Component, Debug, Copy, Clone)]
pub struct Score(pub u32);

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(hill_score_system.system());
    }
}

fn hill_score_system(
    mut player_query: Query<(&mut Score, &TileLocation), With<Player>>,
    tile_query: Query<(&OrphanComponent<Tile>, &TileLocation), Without<Player>>,
    mut ticks: EventReader<Tick>,
) {
    for _ in ticks.iter().filter(|t| matches!(t, Tick::World)) {
        for (mut score, location) in player_query.iter_mut() {
            if let Some(Tile::Hill) =
                tile_query.iter().find_map(|(t, l)| (l == location).then(|| t))
            {
                score.0 += 1;
            }
        }
    }
}
