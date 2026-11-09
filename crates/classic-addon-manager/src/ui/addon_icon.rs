//! `AddonIcon`: an addon's icon image, or a coloured monogram tile when it has none, plus the icon URL helpers.
use cam_ui::components::style_ext::StyleMergeExt as _;
use cam_ui::http_client;
use cam_ui::theme::{TokensExt, alpha, hue_from_string, oklch, oklcha, radius, shadow_xs};
use gpui_kit::{
    AnyElement, App, FontWeight, IntoElement, ObjectFit, ParentElement, Pixels, RenderOnce, SharedString,
    StyleRefinement, Styled, StyledImage as _, Window, div, img, linear_color_stop, linear_gradient, px,
};

/// Resolves the icon URL of an addon. A catalog `icon` (already a full CDN URL) wins. The GitHub
/// `raw.githubusercontent.com/<repo>/<branch>/icon.png` fallback is used only when `github_fallback` is set (owned
/// addons not yet synced to the catalog) and both `repo` and `branch` are known.
///
/// ```ignore
/// let url = addon_icon_url(manifest.icon.as_deref(), None, None, false);
/// let url = addon_icon_url(addon.icon.as_deref(), Some(&addon.repo), addon.branch.as_deref(), true);
/// ```
pub fn addon_icon_url(
    icon: Option<&str>,
    repo: Option<&str>,
    branch: Option<&str>,
    github_fallback: bool,
) -> Option<String> {
    if let Some(icon) = icon.filter(|s| !s.is_empty()) {
        return Some(icon.to_string());
    }
    if github_fallback
        && let (Some(repo), Some(branch)) = (repo.filter(|s| !s.is_empty()), branch.filter(|s| !s.is_empty()))
    {
        return Some(format!("https://raw.githubusercontent.com/{repo}/{branch}/icon.png"));
    }
    None
}

/// Monogram of an addon name: letters and digits only. One word gives its first two characters, otherwise the first
/// characters of the first two words, and `?` when empty. Not upper-cased (the widget does that).
pub fn addon_initials(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' }).collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    match words.as_slice() {
        [] => "?".to_string(),
        [one] => one.chars().take(2).collect(),
        [a, b, ..] => a.chars().take(1).chain(b.chars().take(1)).collect(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AddonIconSize {
    /// 36px, 14px text, large corner radius.
    Sm,
    /// 40px, 15px text, large corner radius.
    #[default]
    Md,
    /// 56px, 20px text, extra-large corner radius.
    Lg,
}

impl AddonIconSize {
    /// (edge px, corner radius, font px)
    fn metrics(self) -> (f32, Pixels, f32) {
        match self {
            AddonIconSize::Sm => (36.0, radius::LG, 14.0),
            AddonIconSize::Md => (40.0, radius::LG, 15.0),
            AddonIconSize::Lg => (56.0, radius::XL, 20.0),
        }
    }
}

/// An addon's icon image, or a monogram coloured from its name when there is no URL or it fails to load.
///
/// ```ignore
/// AddonIcon::new(&addon.alias, addon_icon_url(addon.icon.as_deref(), None, None, false))
/// AddonIcon::new("Deadly Boss Mods", None).size(AddonIconSize::Lg).opacity(0.9)
/// ```
/// `Styled` is implemented for extras such as `.shadow(..)` and `.opacity(0.9)`.
#[derive(IntoElement)]
pub struct AddonIcon {
    name: SharedString,
    src: Option<SharedString>,
    size: AddonIconSize,
    style: StyleRefinement,
}

impl AddonIcon {
    pub fn new(name: impl Into<SharedString>, src: Option<impl Into<SharedString>>) -> Self {
        Self {
            name: name.into(),
            src: src.map(Into::into).filter(|s: &SharedString| !s.is_empty()),
            size: AddonIconSize::Md,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: AddonIconSize) -> Self {
        self.size = size;
        self
    }
}

impl Styled for AddonIcon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The gradient monogram tile (also the fallback of a failed image).
fn monogram(name: &str, edge: f32, corner: Pixels, font: f32) -> gpui_kit::Div {
    let hue = hue_from_string(name) as f32;
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(edge))
        .rounded(corner)
        .border_1()
        .border_color(oklcha(0.6, 0.1, hue, 0.28))
        .bg(linear_gradient(
            140.0,
            linear_color_stop(oklch(0.36, 0.09, hue), 0.0),
            linear_color_stop(oklch(0.24, 0.06, hue), 1.0),
        ))
        .text_color(oklch(0.88, 0.1, hue))
        .text_size(px(font))
        .line_height(px(font * 1.5))
        .font_weight(FontWeight::SEMIBOLD)
        .child(SharedString::from(addon_initials(name).to_uppercase()))
}

impl RenderOnce for AddonIcon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (edge, corner, font) = self.size.metrics();
        match self.src {
            Some(src) => {
                http_client::install(cx);
                let name = self.name.clone();
                let inner = px(edge - 2.0);
                let image: AnyElement = img(src)
                    .size(inner)
                    .rounded(corner - px(1.0))
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(move || monogram(&name, edge - 2.0, corner - px(1.0), font).into_any_element())
                    .into_any_element();
                div()
                    .flex()
                    .flex_none()
                    .size(px(edge))
                    .rounded(corner)
                    .border_1()
                    .border_color(alpha(t.border, 0.5))
                    .shadow(shadow_xs())
                    .merge_style(&self.style)
                    .child(image)
                    .into_any_element()
            }
            None => monogram(&self.name, edge, corner, font).merge_style(&self.style).into_any_element(),
        }
    }
}
