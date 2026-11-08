//! Embedded assets (`assets/`: icons, fonts, images). Debug builds read from disk, release builds embed.
use std::borrow::Cow;

use anyhow::Result;
use gpui_kit::{AssetSource, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[exclude = "*.md"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter().filter(|p| p.starts_with(path)).map(|p| SharedString::from(p.to_string())).collect())
    }
}

/// Registers the embedded TTFs with gpui's text system (Geist + Geist Mono, static weight instances).
pub fn register_fonts(cx: &gpui_kit::App) -> Result<()> {
    let mut fonts = Vec::new();
    for path in Assets::iter().filter(|p| p.starts_with("fonts/") && p.ends_with(".ttf")) {
        if let Some(f) = Assets::get(&path) {
            fonts.push(f.data);
        }
    }
    cx.text_system().add_fonts(fonts)
}

/// Asset paths of bundled images, for `gpui_kit::img(...)`.
pub mod images {
    pub const LOGO_WIDE_1X: &str = "images/aac-logo-wide_1x.png";
    pub const LOGO_WIDE_2X: &str = "images/aac-logo-wide_2x.png";
    pub const AAC_LOGO: &str = "images/aac-logo.png";
    pub const ALT_AAC_LOGO: &str = "images/alt_aac-logo.png";
    pub const APP_ICON: &str = "images/appicon.png";
    pub const SUPPORT_DARU: &str = "images/support_daru.png";
    pub const SUPPORT_DARU_ALT: &str = "images/support_daru_alt.png";
    pub const SUPPORT_DARU_ALT_SM: &str = "images/support_daru_alt_sm.png";
}
