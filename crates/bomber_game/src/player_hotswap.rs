use crate::{
    log_recoverable_error,
    player_behaviour::{filter_name, Player, PlayerName, PlayerNameMarker, MAX_NAME_LENGTH},
    state::Round,
    ExternalCrateComponent,
};
use anyhow::{anyhow, Result};
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use bomber_lib::{wasm_name, world::Ticks};
use wasmtime::{Instance, Store};

pub struct PlayerHotswapPlugin;
pub const MAX_PLAYERS: usize = 12;

/// Handle into a .wasm file, classified by whether or not it misbehaved.
#[derive(Clone, Debug)]
pub enum PlayerHandle {
    ReadyToSpawn(Handle<WasmPlayerAsset>),
    Misbehaved(Handle<WasmPlayerAsset>, String),
    Respawning(Handle<WasmPlayerAsset>, Ticks),
}

impl PlayerHandle {
    pub fn is_ready_to_spawn(&self) -> bool {
        matches!(self, PlayerHandle::ReadyToSpawn(_))
    }

    pub fn inner(&self) -> &Handle<WasmPlayerAsset> {
        match self {
            PlayerHandle::ReadyToSpawn(h) => h,
            PlayerHandle::Misbehaved(h, _) => h,
            PlayerHandle::Respawning(h, _) => h,
        }
    }

    pub fn invalidate(&mut self, reason: String) {
        *self = PlayerHandle::Misbehaved(self.inner().clone(), reason);
    }
}

/// Dynamic list of handles into `.wasm` files, which is updated every frame
/// to match the `.wasm` files under the hotswap folder. Other systems watch
/// for changes to this resource in order to react to players being added and
/// removed from the game.
pub struct PlayerHandles(pub Vec<PlayerHandle>);

#[derive(Debug, TypeUuid)]
#[uuid = "6d74e1ac-79d0-48a9-8fbf-5e1fea758815"]
pub struct WasmPlayerAsset {
    /// Raw `wasm` bytes, whether in binary precompiled `.wasm` format or textual
    /// `.wat` representation (wasmtime can process both).
    pub bytes: Vec<u8>,
}

impl Plugin for PlayerHotswapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerHandles(vec![]))
            .insert_resource(AssetServerSettings { watch_for_changes: true, ..default() })
            .add_asset::<WasmPlayerAsset>()
            .init_asset_loader::<WasmPlayerLoader>()
            .add_system(live_brain_reload_system.chain(log_recoverable_error))
            .add_system(unban_system)
            .add_startup_system(setup)
            .add_system(hotswap_system);
    }
}

#[derive(Default)]
pub struct WasmPlayerLoader;

impl AssetLoader for WasmPlayerLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let wasm_player_asset = WasmPlayerAsset { bytes: bytes.into() };
            load_context.set_default_asset(LoadedAsset::new(wasm_player_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wasm", "wat"]
    }
}

fn setup(asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap()
}

/// Maintains the `PlayerHandles` resource in sync with the files in the hotswap folder.
fn hotswap_system(
    asset_server: Res<AssetServer>,
    mut handles: ResMut<PlayerHandles>,
    round: Res<Round>,
) {
    let mut new_handles = asset_server.load_folder(format!("rounds/{}", round.0)).unwrap();
    // Remove any handles associated to files that have disappeared from the folder
    handles.0.retain(|h| new_handles.iter().any(|new| new.id == h.inner().id));
    // Add any handles that aren't already present and misbehaving
    new_handles.retain(|h| handles.0.iter().all(|old| old.inner().id != h.id));
    handles.0.extend(new_handles.into_iter().map(|new| PlayerHandle::ReadyToSpawn(new.typed())));
    handles.0.truncate(MAX_PLAYERS);
}

/// Keeps characters up to date with their most recent WASM AI.
///
/// Note that this supports changing name live, but not teams, out of fairness. Teams can be changed
/// on death/respawn though.
fn live_brain_reload_system(
    assets: Res<Assets<WasmPlayerAsset>>,
    wasm_engine: Res<wasmtime::Engine>,
    mut players: Query<
        (
            Entity,
            &mut ExternalCrateComponent<Instance>,
            &mut ExternalCrateComponent<Store<()>>,
            &mut PlayerName,
            &Handle<WasmPlayerAsset>,
        ),
        With<Player>,
    >,
    mut player_name_text: Query<(&mut Text, &Parent), With<PlayerNameMarker>>,
    mut events: EventReader<AssetEvent<WasmPlayerAsset>>,
) -> Result<()> {
    let changed_handles = events.iter().filter_map(|e| match e {
        AssetEvent::Modified { handle } => Some(handle),
        _ => None,
    });

    for handle in changed_handles {
        for (entity, mut instance, mut store, mut player_name, player_handle) in players.iter_mut()
        {
            if handle.id == player_handle.id {
                let wasm_bytes = assets
                    .get(handle)
                    .ok_or_else(|| anyhow!("Wasm asset not found at runtime"))?
                    .bytes
                    .clone();
                let module = wasmtime::Module::new(&wasm_engine, wasm_bytes)?;
                let mut store = &mut **store;
                **instance = wasmtime::Instance::new(&mut store, &module, &[])?;

                if let Ok(name) = wasm_name(store, &instance) {
                    let name = filter_name(&name, MAX_NAME_LENGTH);
                    player_name.0 = name.clone();
                    for mut text in player_name_text
                        .iter_mut()
                        .filter_map(|(text, p)| (p.get() == entity).then_some(text))
                    {
                        text.sections[0].value = name.clone();
                    }
                }
            }
        }
    }

    Ok(())
}

/// Returns "banned" (misbehaving) players to the arena when a new AI is uploaded for them,
/// assuming that the upload fixes the issue.
fn unban_system(
    mut handles: ResMut<PlayerHandles>,
    mut events: EventReader<AssetEvent<WasmPlayerAsset>>,
) {
    let changed_handles = events.iter().filter_map(|e| match e {
        AssetEvent::Modified { handle } => Some(handle),
        _ => None,
    });
    for changed_handle in changed_handles {
        if let Some(handle) = handles.0.iter_mut().find(|h| h.inner() == changed_handle) {
            if matches!(handle, PlayerHandle::Misbehaved(..)) {
                *handle = PlayerHandle::ReadyToSpawn(changed_handle.clone())
            }
        }
    }
}
