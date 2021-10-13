use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

pub struct PlayerHotswapPlugin;

/// Dynamic list of handles into `.wasm` files, which is updated every frame
/// to match the `.wasm` files under the hotswap folder. Other systems watch
/// for changes to this resource in order to react to players being added and
/// removed from the game.
pub struct PlayerHandles(pub Vec<Handle<WasmPlayerAsset>>);

#[derive(Debug, TypeUuid)]
#[uuid = "6d74e1ac-79d0-48a9-8fbf-5e1fea758815"]
pub struct WasmPlayerAsset {
    /// Raw `wasm` bytes, whether in binary precompiled `.wasm` format or textual
    /// `.wat` representation (wasmtime can process both).
    pub bytes: Vec<u8>,
}

impl Plugin for PlayerHotswapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PlayerHandles(vec![]))
            .add_asset::<WasmPlayerAsset>()
            .init_asset_loader::<WasmPlayerLoader>()
            .add_system(hotswap_system.system());
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
    let untyped_handles = asset_server.load_folder("players").unwrap();
    handles.0 = untyped_handles.into_iter().map(|h| h.typed()).collect();
}
