use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

pub struct HeroHotswapPlugin;
pub struct HeroHandles(pub Vec<Handle<WasmHeroAsset>>);

#[derive(Debug, TypeUuid)]
#[uuid = "6d74e1ac-79d0-48a9-8fbf-5e1fea758815"]
pub struct WasmHeroAsset {
    pub bytes: Vec<u8>,
}

impl Plugin for HeroHotswapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .insert_resource(HeroHandles(vec![]))
            .add_asset::<WasmHeroAsset>()
            .init_asset_loader::<WasmHeroLoader>()
            .add_system(hotswap_system.system());
    }
}

#[derive(Default)]
pub struct WasmHeroLoader;

impl AssetLoader for WasmHeroLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let wasm_hero_asset = WasmHeroAsset{ bytes: bytes.into() };
            load_context.set_default_asset(LoadedAsset::new(wasm_hero_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wasm", "wat"]
    }
}

fn hotswap_system (
    asset_server: Res<AssetServer>,
    mut handles: ResMut<HeroHandles>,
) {
    let untyped_handles = asset_server.load_folder("heroes").unwrap();
    handles.0 = untyped_handles.into_iter().map(|h| h.typed()).collect();
}
