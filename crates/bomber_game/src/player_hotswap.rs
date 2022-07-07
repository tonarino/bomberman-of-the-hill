use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use bomber_lib::world::Ticks;

pub struct PlayerHotswapPlugin;

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
            .add_asset::<WasmPlayerAsset>()
            .init_asset_loader::<WasmPlayerLoader>()
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

/// Maintains the `PlayerHandles` resource in sync with the files in the hotswap folder.
fn hotswap_system(asset_server: Res<AssetServer>, mut handles: ResMut<PlayerHandles>) {
    let mut new_handles = asset_server.load_folder("rounds/1").unwrap();
    // Remove any handles associated to files that have disappeared from the folder
    handles.0.retain(|h| new_handles.iter().any(|new| new.id == h.inner().id));
    // Add any handles that aren't already present and misbehaving
    new_handles.retain(|h| handles.0.iter().all(|old| old.inner().id != h.id));
    handles.0.extend(new_handles.into_iter().map(|new| PlayerHandle::ReadyToSpawn(new.typed())));
}
