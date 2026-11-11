use gpui_kit::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets"]
#[prefix = "assets/"]
struct EmbeddedAssets;

pub struct KbuiAssets;

impl AssetSource for KbuiAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = EmbeddedAssets::get(path) {
            return Ok(Some(file.data));
        }
        Ok(gpui_kit::assets::Assets::get(path).map(|file| file.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let prefix = if path.ends_with('/') || path.is_empty() {
            path.to_string()
        } else {
            format!("{path}/")
        };
        let mut assets = gpui_kit::assets::Assets.list(path)?;
        assets.extend(
            EmbeddedAssets::iter()
                .filter(|name| name.starts_with(&prefix))
                .map(|name| SharedString::from(name.into_owned())),
        );
        assets.sort();
        assets.dedup();
        Ok(assets)
    }
}
