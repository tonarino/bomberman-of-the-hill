use anyhow::Result;
use bevy::prelude::*;

use crate::{
    log_unrecoverable_error_and_panic,
    player_behaviour::PlayerName,
    rendering::{PLAYER_HEIGHT_PX, PLAYER_WIDTH_PX, VICTORY_SCREEN_ITEMS_Z, VICTORY_SCREEN_Z},
    score::Score,
    state::{AppState, AppStateTimer},
};

pub struct VictoryScreenPlugin;

struct VictoryScreen;
struct CountdownText;

struct Fonts {
    mono: Handle<Font>,
}

impl Plugin for VictoryScreenPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let asset_server =
            app.world().get_resource::<AssetServer>().expect("Asset server not found");

        let fonts = Fonts { mono: asset_server.load("fonts/space_mono_400.ttf") };
        app.insert_resource(fonts);
        app.add_system_set(
            SystemSet::on_enter(AppState::VictoryScreen).with_system(setup.system()),
        )
        .add_system_set(SystemSet::on_update(AppState::VictoryScreen).with_system(
            countdown_text_system.system().chain(log_unrecoverable_error_and_panic.system()),
        ))
        .add_system_set(
            SystemSet::on_exit(AppState::VictoryScreen)
                .with_system(cleanup.system().chain(log_unrecoverable_error_and_panic.system())),
        );
    }
}

fn setup(
    player_query: Query<(&PlayerName, &Score, &Handle<ColorMaterial>)>,
    fonts: Res<Fonts>,
    windows: Res<Windows>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let window = windows.get_primary().unwrap();

    // Fill the background in a transparent black.
    commands
        .spawn()
        .insert(VictoryScreen)
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgba(0.0, 0.0, 0.0, 0.95).into()),
            sprite: Sprite::new(Vec2::new(window.width(), window.height())),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, VICTORY_SCREEN_Z)),
            ..Default::default()
        })
        .with_children(|parent| {
            spawn_podium(parent, player_query, &fonts);
            spawn_countdown_text(parent, &fonts);
        });
}

fn spawn_podium(
    parent: &mut ChildBuilder,
    player_query: Query<(&PlayerName, &Score, &Handle<ColorMaterial>)>,
    fonts: &Fonts,
) {
    // TODO(ryo): Handle a tie.
    let no1_player = player_query.iter().max_by_key(|(_, Score(point), _)| point);
    if let Some((PlayerName(name), Score(score), material)) = no1_player {
        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text(&format!("#1 {}", name), 60.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, 80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });

        // The player avatar doubled in size.
        parent.spawn().insert_bundle(SpriteBundle {
            material: material.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, VICTORY_SCREEN_ITEMS_Z)),
            sprite: Sprite::new(Vec2::new(PLAYER_WIDTH_PX, PLAYER_HEIGHT_PX) * 2.0),
            ..Default::default()
        });

        parent.spawn().insert_bundle(Text2dBundle {
            text: mono_text(&format!("{} points", score), 30.0, fonts),
            transform: Transform::from_translation(Vec3::new(0.0, -80.0, VICTORY_SCREEN_ITEMS_Z)),
            ..Default::default()
        });
    }
}

fn spawn_countdown_text(parent: &mut ChildBuilder, fonts: &Fonts) {
    parent.spawn().insert_bundle(Text2dBundle {
        text: mono_text("Next round in...", 30.0, fonts),
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
    timer_query: Query<&Timer, With<AppStateTimer>>,
    mut count_down_text_query: Query<&mut Text, With<CountdownText>>,
) -> Result<()> {
    let timer = timer_query.single()?;
    let remaining = timer.duration() - timer.elapsed();

    let mut count_down_text = count_down_text_query.single_mut()?;
    count_down_text.sections[0].value = format!("{}", remaining.as_secs());

    Ok(())
}

fn cleanup(
    victory_screen_query: Query<Entity, With<VictoryScreen>>,
    mut commands: Commands,
) -> Result<()> {
    let entity = victory_screen_query.single()?;
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
