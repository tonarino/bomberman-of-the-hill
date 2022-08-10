use anyhow::Result;
use bevy::prelude::*;

use crate::{
    audio::SoundEffects,
    log_unrecoverable_error_and_panic,
    player_behaviour::{PlayerName, Team},
    rendering::{PLAYER_HEIGHT_PX, PLAYER_WIDTH_PX, VICTORY_SCREEN_ITEMS_Z, VICTORY_SCREEN_Z},
    score::Score,
    state::{AppState, Round, RoundTimer},
};

pub struct VictoryScreenPlugin;

#[derive(Component)]
struct VictoryScreen;
#[derive(Component)]
struct CountdownText;

struct Fonts {
    mono: Handle<Font>,
}

impl Plugin for VictoryScreenPlugin {
    fn build(&self, app: &mut App) {
        let asset_server = app.world.get_resource::<AssetServer>().expect("Asset server not found");

        let fonts = Fonts { mono: asset_server.load("fonts/space_mono_400.ttf") };
        app.insert_resource(fonts);
        app.add_system_set(SystemSet::on_enter(AppState::VictoryScreen).with_system(setup))
            .add_system_set(
                SystemSet::on_update(AppState::VictoryScreen)
                    .with_system(countdown_text_system.chain(log_unrecoverable_error_and_panic)),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::VictoryScreen)
                    .with_system(cleanup.chain(log_unrecoverable_error_and_panic)),
            );
    }
}

fn setup(
    player_query: Query<(&PlayerName, &Score, &Team)>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    fonts: Res<Fonts>,
    windows: Res<Windows>,
    round: Res<Round>,
    audio: Res<Audio>,
    sound_effects: Res<SoundEffects>,
    mut commands: Commands,
) {
    let window = windows.get_primary().unwrap();
    audio.play(sound_effects.win.clone());

    // Fill the background in a transparent black.
    commands
        .spawn()
        .insert(VictoryScreen)
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.0, 0.0, 0.0, 0.95),
                custom_size: Some(Vec2::new(window.width(), window.height())),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, VICTORY_SCREEN_Z)),
            ..Default::default()
        })
        .with_children(|parent| {
            spawn_podium(parent, player_query, &asset_server, &mut texture_atlases, &fonts);
            spawn_countdown_text(parent, &fonts, &round);
        });
}

fn spawn_podium(
    parent: &mut ChildBuilder,
    player_query: Query<(&PlayerName, &Score, &Team)>,
    asset_server: &AssetServer,
    texture_atlases: &mut Assets<TextureAtlas>,
    fonts: &Fonts,
) {
    // TODO(ryo): Handle a tie.
    let no1_player = player_query
        .iter()
        .filter(|(_, Score(point), _)| *point > 0)
        .max_by_key(|(_, Score(point), _)| point);
    if let Some((PlayerName(name), Score(score), team)) = no1_player {
        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text(&format!("#1 {} from team {}", name, team.name), 60.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, 80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });

        let texture_handle = asset_server.load("graphics/Sprites/Bomberman/sheet.png");
        let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(21.0, 32.0), 5, 4);
        let texture_atlas_handle = texture_atlases.add(texture_atlas);

        // The player avatar doubled in size.
        parent.spawn().insert_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: 2,
                color: team.color,
                custom_size: Some(Vec2::new(PLAYER_WIDTH_PX, PLAYER_HEIGHT_PX) * 2.0),
                ..Default::default()
            },
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, VICTORY_SCREEN_ITEMS_Z)),
            ..default()
        });

        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text(&format!("{} points", score), 30.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, -80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });
    } else {
        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text("Nobody got any points :(", 60.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, 80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });
        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text("Good luck and get to the hill!", 30.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, -80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });
    }
}

fn spawn_countdown_text(parent: &mut ChildBuilder, fonts: &Fonts, round: &Round) {
    parent.spawn().insert_bundle(Text2dBundle {
        text: mono_text(&format!("Next round ({}) in...", round.0 + 1), 30.0, fonts),
        transform: Transform::from_translation(Vec3::new(0.0, -200.0, VICTORY_SCREEN_ITEMS_Z)),
        ..Default::default()
    });

    parent.spawn().insert(CountdownText).insert_bundle(Text2dBundle {
        text: mono_text("", 60.0, fonts),
        transform: Transform::from_translation(Vec3::new(0.0, -240.0, VICTORY_SCREEN_ITEMS_Z)),
        ..Default::default()
    });
}

fn countdown_text_system(
    timer_query: Query<&RoundTimer>,
    mut count_down_text_query: Query<&mut Text, With<CountdownText>>,
) -> Result<()> {
    let RoundTimer(timer) = timer_query.single();
    let remaining = timer.duration() - timer.elapsed();

    let mut count_down_text = count_down_text_query.single_mut();
    count_down_text.sections[0].value = format!("{}", remaining.as_secs());

    Ok(())
}

fn cleanup(
    victory_screen_query: Query<Entity, With<VictoryScreen>>,
    mut commands: Commands,
) -> Result<()> {
    let entity = victory_screen_query.single();
    commands.entity(entity).despawn_recursive();

    Ok(())
}

fn mono_text(text: &str, font_size: f32, fonts: &Fonts) -> Text {
    Text::with_section(
        text,
        TextStyle { font: fonts.mono.clone(), font_size, color: Color::WHITE },
        TextAlignment { vertical: VerticalAlign::Center, horizontal: HorizontalAlign::Center },
    )
}
