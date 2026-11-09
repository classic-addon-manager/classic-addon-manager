//! `spinner()`: gpui-kit's `Spinner` with the Lucide `loader-circle` icon turning at a constant speed. Shown in busy
//! buttons and loading blocks.
use gpui_kit::component::{Sizable as _, spinner::Spinner};
use gpui_kit::px;

use crate::icons;

/// 16 px loader; resize with `.with_size(px(14.0))` (`gpui_kit::component::Sizable`), tint with `.color(..)`.
/// It inherits the parent's text colour otherwise. It is not `Styled`: wrap it in a `div` for margins or opacity.
pub fn spinner() -> Spinner {
    Spinner::new().icon(icons::LOADER_CIRCLE).ease(gpui_kit::linear).with_size(px(16.0))
}
