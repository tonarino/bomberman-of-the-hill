use bevy::prelude::*;
use bevy_egui::{
    egui::{self, epaint::Shadow, style::Widgets, Color32, RichText, Stroke},
    EguiContext, EguiPlugin,
};

use crate::{
    player_behaviour::{KillPlayerEvent, PlayerName, SpawnPlayerEvent},
    score::Score,
    state::{AppState, RoundTimer},
};

pub struct GameUiPlugin;

/// Marker component that identifies a score/name pair as belonging to a dead
/// (despawned) player, so their last score is visible until they respawn.
#[derive(Component)]
struct DeadPlayerScore;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.add_system(dead_player_score_system);
        app.add_system_set(SystemSet::on_update(AppState::InGame).with_system(score_panel_system));
        app.add_startup_system(configure_visuals);
    }
}

fn score_panel_system(
    mut egui_context: ResMut<EguiContext>,
    player_query: Query<(&PlayerName, &Score, Option<&DeadPlayerScore>)>,
    round_timer_query: Query<&RoundTimer>,
) {
    let mut score_entries = player_query.iter().collect::<Vec<_>>();
    // Sort by descending score
    score_entries.sort_by(|(_, Score(a), _), (_, Score(b), _)| b.cmp(a));
    let timer = round_timer_query.single();
    let remaining = timer.0.duration() - timer.0.elapsed();
    let (minutes, seconds) = (remaining.as_secs() / 60, remaining.as_secs() % 60);

    egui::SidePanel::left("Player Score").resizable(false).show(egui_context.ctx_mut(), |ui| {
        ui.vertical_centered_justified(|ui| {
            let label_text =
                RichText::new(format!("Round ends in {minutes}:{seconds:02}")).size(25.0);
            ui.label(label_text);
            ui.separator();
            ui.heading(RichText::new("Player Score").strong());
            egui::Grid::new("Score Grid").striped(true).show(ui, |ui| {
                for (PlayerName(name), score, dead_marker) in score_entries.iter() {
                    ui.colored_label(
                        if dead_marker.is_some() {
                            tonari_color::STRAWBERRY_LETTER_23
                        } else {
                            tonari_color::MIDNIGHT
                        },
                        name,
                    );
                    ui.label(format!(
                        "{}{}",
                        score.0,
                        if dead_marker.is_some() { " (Dead)" } else { "" }
                    ));
                    ui.end_row();
                }
                ui.allocate_space(ui.available_size());
            });
        });
    });
}

fn dead_player_score_system(
    mut spawn_events: EventReader<SpawnPlayerEvent>,
    mut kill_events: EventReader<KillPlayerEvent>,
    mut commands: Commands,
    dead_player_scores: Query<(Entity, &DeadPlayerScore, &PlayerName)>,
) {
    for SpawnPlayerEvent(name) in spawn_events.iter() {
        if let Some(entity) =
            dead_player_scores.iter().find_map(|(e, _, n)| (n.0 == name.0).then(|| e))
        {
            commands.entity(entity).despawn_recursive();
        }
    }
    for KillPlayerEvent(_, name, score) in kill_events.iter() {
        // The player themselves will be despawned this frame, but we instead insert a score marker that will persist
        // until they despawn.
        commands.spawn().insert(name.clone()).insert(*score).insert(DeadPlayerScore);
    }
}

fn configure_visuals(mut egui_ctx: ResMut<EguiContext>) {
    let faded_little_dragon = Color32::from_rgb(102, 178, 162);
    let mut widgets = Widgets::light();
    widgets.noninteractive.bg_fill = tonari_color::LITTLE_DRAGON;
    widgets.noninteractive.bg_stroke = Stroke { color: tonari_color::PURPLE_RAIN, width: 1.0 };
    widgets.noninteractive.fg_stroke = Stroke { color: tonari_color::PURPLE_RAIN, width: 3.0 };

    let visuals = egui::Visuals {
        dark_mode: false,
        window_rounding: 0.0.into(),
        widgets,
        window_shadow: Shadow { extrusion: 0.0, color: tonari_color::GREEN_DAY },
        faint_bg_color: faded_little_dragon,
        ..Default::default()
    };
    egui_ctx.ctx_mut().set_visuals(visuals);
}

#[allow(unused)]
pub mod tonari_color {
    use super::egui::Color32;
    pub const BLUE_MOON: Color32 = Color32::from_rgb(50, 108, 242);
    pub const GREEN_DAY: Color32 = Color32::from_rgb(38, 201, 140);
    pub const THE_WHITE_STRIPES: Color32 = Color32::from_rgb(254, 251, 244);
    pub const RECYCLED_AIR: Color32 = Color32::from_rgb(248, 249, 234);
    pub const CHROMEO: Color32 = Color32::from_rgb(211, 210, 215);
    pub const DEEP_PURPLE: Color32 = Color32::from_rgb(54, 60, 89);
    pub const LOVE: Color32 = Color32::from_rgb(218, 53, 117);
    pub const RED_HOT_CHILI_PEPPERS: Color32 = Color32::from_rgb(236, 91, 99);
    pub const STRAWBERRY_LETTER_23: Color32 = Color32::from_rgb(235, 98, 81);
    pub const DJ_MUSTARD: Color32 = Color32::from_rgb(243, 174, 86);
    pub const YELLOW_SUBMARINE: Color32 = Color32::from_rgb(246, 201, 98);
    pub const LITTLE_DRAGON: Color32 = Color32::from_rgb(122, 198, 182);
    pub const MY_LIFE_IS_SO_BLUE: Color32 = Color32::from_rgb(18, 47, 161);
    pub const PURPLE_RAIN: Color32 = Color32::from_rgb(84, 68, 150);
    pub const PINK_FLOYD: Color32 = Color32::from_rgb(247, 203, 210);
    pub const YOSHIMI_BATTLES_THE_PINK_ROBOTS: Color32 = Color32::from_rgb(255, 150, 148);
    pub const O_SOLE_MIO: Color32 = Color32::from_rgb(252, 254, 164);
    pub const MINT_CONDITION: Color32 = Color32::from_rgb(212, 250, 204);
    pub const LILAC_WINE: Color32 = Color32::from_rgb(185, 182, 225);
    pub const RUSTIE: Color32 = Color32::from_rgb(202, 182, 173);
    pub const JAMES_BROWN: Color32 = Color32::from_rgb(146, 129, 122);
    pub const ANOTHER_GREEN_WORLD: Color32 = Color32::from_rgb(178, 195, 145);
    pub const MIDNIGHT: Color32 = Color32::from_rgb(76, 81, 105);
    pub const PURE_SHORES: Color32 = Color32::from_rgb(255, 255, 255);
}
