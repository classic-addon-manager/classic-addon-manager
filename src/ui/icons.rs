//! Lucide icons, stored as SVGs in `assets/icons/`.
//!
//! * `icons::DOWNLOAD` etc.: typed constants (generated into `icons_generated.rs` from `assets/icons/*.svg`).
//! * `lucide("download")`: by kebab-case lucide name.
//! * Both are `gpui_kit::component::IconNamed`, so they work anywhere gpui-component takes an icon:
//!   `Button::new("x").icon(icons::DOWNLOAD)`, `Icon::new(icons::DOWNLOAD)`.
//! * `icon(icons::DOWNLOAD)` -> `Icon` pre-sized to 16px. Add more with `tools/gen_icons.py <Name>`.
use gpui_kit::SharedString;
use gpui_kit::component::{Icon, IconNamed};

pub use super::icons_generated::*;

/// An icon in `assets/icons/<name>.svg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lucide(pub &'static str);

impl IconNamed for Lucide {
    fn path(self) -> SharedString {
        SharedString::from(format!("icons/{}.svg", self.0))
    }
}

/// `lucide("circle-check")`
pub const fn lucide(name: &'static str) -> Lucide {
    Lucide(name)
}

/// 16x16 icon. Resize it with `.size_3()`, `.size_3p5()`, `.size_5()` and colour it with `.text_color(..)`.
pub fn icon(name: impl IconNamed) -> Icon {
    use gpui_kit::Styled;
    Icon::new(name).size_4()
}
