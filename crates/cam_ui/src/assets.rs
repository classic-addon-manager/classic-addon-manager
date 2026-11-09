//! The kit's own embedded assets (`assets/` of this crate): the Lucide icons and the Geist fonts. Debug builds read
//! them from disk, release builds embed them. The app serves these next to its own assets, see [`Assets::load`].
use std::borrow::Cow;

use anyhow::Result;
use gpui_kit::{App, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

impl Assets {
    /// The asset at `path` (`icons/x.svg`), for the app's `AssetSource`.
    pub fn load(path: &str) -> Option<Cow<'static, [u8]>> {
        Self::get(path).map(|f| f.data)
    }

    /// Every asset path that starts with `path`, for the app's `AssetSource`.
    pub fn list(path: &str) -> impl Iterator<Item = SharedString> {
        Self::iter().filter(move |p| p.starts_with(path)).map(|p| SharedString::from(p.to_string()))
    }
}

/// Registers the embedded TTFs with gpui's text system (Geist + Geist Mono, static weight instances).
pub(crate) fn register_fonts(cx: &App) -> Result<()> {
    let fonts = Assets::iter()
        .filter(|p| p.starts_with("fonts/") && p.ends_with(".ttf"))
        .filter_map(|p| Assets::get(&p))
        .map(|f| f.data)
        .collect();
    cx.text_system().add_fonts(fonts)
}
