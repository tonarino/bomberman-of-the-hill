use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

pub struct PlayerHotswapPlugin;
pub struct PlayerHandles(pub Vec<Handle<WasmPlayerAsset>>);

#[derive(Debug, TypeUuid)]
#[uuid = "6d74e1ac-79d0-48a9-8fbf-5e1fea758815"]
pub struct WasmPlayerAsset {
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
            let wasm_player_asset = WasmPlayerAsset {
                bytes: bytes.into(),
            };
            load_context.set_default_asset(LoadedAsset::new(wasm_player_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wasm", "wat"]
    }
}

fn hotswap_system(asset_server: Res<AssetServer>, mut handles: ResMut<PlayerHandles>) {
    let untyped_handles = asset_server.load_folder("players").unwrap();
    handles.0 = untyped_handles.into_iter().map(|h| h.typed()).collect();
}
