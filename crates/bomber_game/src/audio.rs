use bevy::prelude::*;

pub struct SoundEffects {
    pub explosion: Handle<AudioSource>,
    pub drop: Handle<AudioSource>,
    pub spawn: Handle<AudioSource>,
    pub death: Handle<AudioSource>,
    pub powerup: Handle<AudioSource>,
    pub win: Handle<AudioSource>,
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        let asset_server =
            app.world.get_resource::<AssetServer>().expect("Failed to retrieve asset server");
        let sound_effects = SoundEffects {
            explosion: asset_server.load("audio/sound_effects/PP_Weapon_Shoot_Big.wav"),
            drop: asset_server.load("audio/sound_effects/bomb-drop.mp3"),
            spawn: asset_server.load("audio/sound_effects/PP_Summon.wav"),
            death: asset_server.load("audio/sound_effects/LQ_Lose_Sting_01.wav"),
            powerup: asset_server.load("audio/sound_effects/PP_Collect_Item_1_2.wav"),
            win: asset_server.load("audio/sound_effects/FA_Win_Stinger_1_1.wav"),
        };
        app.insert_resource(sound_effects);
    }
}
