//! The app's asset source: its own images (`assets/` of this crate) plus the icons and fonts of `cam_ui`. Debug
//! builds read them from disk, release builds embed them.
use std::borrow::Cow;

use anyhow::Result;
use gpui_kit::{AssetSource, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
struct Images;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Images::get(path).map(|f| f.data).or_else(|| cam_ui::assets::Assets::load(path)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let own = Images::iter().filter(|p| p.starts_with(path)).map(|p| SharedString::from(p.to_string()));
        Ok(own.chain(cam_ui::assets::Assets::list(path)).collect())
    }
}

/// Asset paths of bundled images, for `gpui_kit::img(...)`.
pub mod images {
    pub const LOGO_WIDE_2X: &str = "images/aac-logo-wide_2x.png";
    pub const SUPPORT_DARU: &str = "images/support_daru.png";
    pub const SUPPORT_DARU_ALT_SM: &str = "images/support_daru_alt_sm.png";
}
