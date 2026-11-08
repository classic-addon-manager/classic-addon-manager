//! Dark theme with eight accent colours, and its mapping onto gpui-component's `Theme`.
//!
//! * `cx.tokens()` returns the semantic colours (`background`, `card`, `primary`, `border`, ...). They are the only
//!   colour source of the app: `apply_accent` derives gpui-component's `ThemeColor` from them.
//! * `theme::radius::{SM, MD, LG, XL}` are the corner radii of the app (6.4, 8.4, 10.4 and 14.4 px, derived from a
//!   base radius of 0.65 rem). Use these constants instead of gpui's own radii.
//! * `theme::tw(Tw::Emerald, 500)` looks up a colour of the Tailwind default palette for hard-coded colours.
//! * `TwText` sets text size and line height in one call, and selects the monospace font.
//! * `shadow_xs()` up to `shadow_2xl()` are the drop shadows, `lerp_color` and `blend` the colour mixing helpers.

pub mod palette;

use gpui_kit::component::{Theme, ThemeColor, ThemeMode};
use gpui_kit::{
    App, BoxShadow, Global, Hsla, Pixels, Rgba, SharedString, StatefulInteractiveElement, Styled, hsla, point, px, rems,
};

pub use palette::{Tw, tw};

use crate::backend::config::AccentColorId;

/// Converts an OKLCH colour (`l` in 0..1, `h` in degrees) to gpui `Hsla`.
pub fn oklch(l: f32, c: f32, h: f32) -> Hsla {
    oklcha(l, c, h, 1.0)
}

pub fn oklcha(l: f32, c: f32, h: f32, alpha: f32) -> Hsla {
    let (l, c, h) = (l as f64, c as f64, (h as f64).to_radians());
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    let r = 4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_;
    let g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_;
    let bl = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_;
    let gamma = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.0031308 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    };
    Rgba { r: gamma(r) as f32, g: gamma(g) as f32, b: gamma(bl) as f32, a: alpha }.into()
}

/// `color` with its alpha replaced: `alpha(primary, 0.5)` is the primary colour at half opacity.
pub fn alpha(color: Hsla, a: f32) -> Hsla {
    Hsla { a, ..color }
}

/// Corner radii of the app, derived from a base radius of 0.65 rem.
pub mod radius {
    use gpui_kit::{Pixels, px};
    pub const BASE: f32 = 0.65 * 16.0; // 10.4 px
    pub const SM: Pixels = px(BASE - 4.0); // 6.4 px
    pub const MD: Pixels = px(BASE - 2.0); // 8.4 px
    pub const LG: Pixels = px(BASE); // 10.4 px
    pub const XL: Pixels = px(BASE + 4.0); // 14.4 px
    /// Extra large radius, 16 px.
    pub const XXL: Pixels = px(16.0);
    /// Small plain radius, 0.25 rem (4 px).
    pub const DEFAULT: Pixels = px(4.0);
}

// ---------------------------------------------------------------------------------------------------------
// Accent presets
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct AccentPreset {
    pub id: AccentColorId,
    pub label: &'static str,
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

pub const ACCENT_PRESETS: [AccentPreset; 8] = [
    AccentPreset { id: AccentColorId::Gold, label: "Gold", l: 0.83, c: 0.095, h: 80.0 },
    AccentPreset { id: AccentColorId::Amber, label: "Amber", l: 0.8, c: 0.14, h: 65.0 },
    AccentPreset { id: AccentColorId::Rose, label: "Rose", l: 0.72, c: 0.16, h: 15.0 },
    AccentPreset { id: AccentColorId::Violet, label: "Violet", l: 0.7, c: 0.17, h: 295.0 },
    AccentPreset { id: AccentColorId::Blue, label: "Blue", l: 0.7, c: 0.14, h: 250.0 },
    AccentPreset { id: AccentColorId::Cyan, label: "Cyan", l: 0.78, c: 0.11, h: 195.0 },
    AccentPreset { id: AccentColorId::Emerald, label: "Emerald", l: 0.78, c: 0.14, h: 155.0 },
    AccentPreset { id: AccentColorId::Slate, label: "Slate", l: 0.8, c: 0.02, h: 260.0 },
];

impl AccentColorId {
    pub fn preset(self) -> &'static AccentPreset {
        ACCENT_PRESETS.iter().find(|p| p.id == self).unwrap()
    }

    /// Swatch colour that previews this accent.
    pub fn swatch(self) -> Hsla {
        let p = self.preset();
        oklch(p.l, p.c, p.h)
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------------------------------------

/// Semantic colours of the dark theme with the active accent applied.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub chart: [Hsla; 5],
    pub sidebar: Hsla,
    pub sidebar_foreground: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_border: Hsla,
}

impl Global for Tokens {}

impl Tokens {
    pub fn dark(accent: AccentColorId) -> Self {
        let p = accent.preset();
        let primary = oklch(p.l, p.c, p.h);
        let primary_foreground = oklch(0.205, 0.0, 0.0);
        let chart = std::array::from_fn(|i| {
            let s = i as f32;
            oklch(p.l - s * 0.08, p.c + s * 0.005, p.h)
        });
        Self {
            background: oklch(0.145, 0.0, 0.0),
            foreground: oklch(0.985, 0.0, 0.0),
            card: oklch(0.205, 0.0, 0.0),
            card_foreground: oklch(0.985, 0.0, 0.0),
            popover: oklch(0.269, 0.0, 0.0),
            popover_foreground: oklch(0.985, 0.0, 0.0),
            primary,
            primary_foreground,
            secondary: oklch(0.269, 0.0, 0.0),
            secondary_foreground: oklch(0.985, 0.0, 0.0),
            muted: oklch(0.269, 0.0, 0.0),
            muted_foreground: oklch(0.708, 0.0, 0.0),
            accent: oklch(0.371, 0.0, 0.0),
            accent_foreground: oklch(0.985, 0.0, 0.0),
            destructive: oklch(0.704, 0.191, 22.216),
            destructive_foreground: oklch(0.985, 0.0, 0.0),
            border: oklch(0.275, 0.0, 0.0),
            input: oklch(0.325, 0.0, 0.0),
            ring: primary,
            chart,
            sidebar: oklch(0.205, 0.0, 0.0),
            sidebar_foreground: oklch(0.985, 0.0, 0.0),
            sidebar_primary: primary,
            sidebar_primary_foreground: oklch(0.985, 0.0, 0.0),
            sidebar_accent: oklch(0.269, 0.0, 0.0),
            sidebar_accent_foreground: oklch(0.985, 0.0, 0.0),
            sidebar_border: oklch(0.275, 0.0, 0.0),
        }
    }

    /// Pure white.
    pub fn white() -> Hsla {
        hsla(0.0, 0.0, 1.0, 1.0)
    }
}

/// `cx.tokens()`: the colours of the running app, on `App` and, through `Deref`, on every `Context`. Panics if
/// `theme::init` has not run.
pub trait TokensExt {
    /// Returns a copy, so a render function can hold its colours while it borrows `cx` mutably. Copying the 31
    /// colours costs about as much as reading them.
    fn tokens(&self) -> Tokens;
}

impl TokensExt for App {
    fn tokens(&self) -> Tokens {
        *self.global::<Tokens>()
    }
}

// ---------------------------------------------------------------------------------------------------------
// Init / accent application
// ---------------------------------------------------------------------------------------------------------

pub const FONT_SANS: &str = "Geist";
pub const FONT_MONO: &str = "Geist Mono";

/// Installs the dark theme: tokens global, gpui-component `Theme` colours, fonts and radii.
pub fn init(cx: &mut App, accent: AccentColorId) {
    Theme::change(ThemeMode::Dark, None, cx);
    apply_accent(cx, accent);
}

/// Applies accent preset `id` at runtime (primary/ring/sidebar-primary/chart) and re-maps the gpui-component theme.
pub fn apply_accent(cx: &mut App, id: AccentColorId) {
    let t = Tokens::dark(id);
    cx.set_global(t);
    // `Theme::update` keeps `theme.colors`, `theme.tokens` and gpui-kit's Base theme (scrollbar/resize handles) in
    // step and refreshes every window, `Theme::global_mut` would leave the scrollbar painting the old colours.
    Theme::update(cx, |theme| map_theme(theme, &t));
}

fn map_theme(theme: &mut Theme, t: &Tokens) {
    let mut c: ThemeColor = *ThemeColor::dark();
    c.background = t.background;
    c.foreground = t.foreground;
    c.border = t.border;
    c.input = t.input;
    c.ring = t.ring;
    c.caret = t.foreground;
    c.selection = alpha(t.primary, 0.35);
    c.accent = t.accent;
    c.accent_foreground = t.accent_foreground;
    c.muted = t.muted;
    c.muted_foreground = t.muted_foreground;
    c.popover = t.popover;
    c.popover_foreground = t.popover_foreground;
    c.primary = t.primary;
    c.primary_foreground = t.primary_foreground;
    c.primary_hover = alpha(t.primary, 0.9);
    c.primary_active = alpha(t.primary, 0.8);
    c.secondary = t.secondary;
    c.secondary_foreground = t.secondary_foreground;
    c.secondary_hover = alpha(t.secondary, 0.8);
    c.secondary_active = alpha(t.secondary, 0.7);
    c.danger = t.destructive;
    c.danger_foreground = t.destructive_foreground;
    c.danger_hover = alpha(t.destructive, 0.9);
    c.danger_active = alpha(t.destructive, 0.8);
    c.link = t.primary;
    c.link_hover = alpha(t.primary, 0.8);
    c.link_active = alpha(t.primary, 0.7);
    c.success = tw(Tw::Emerald, 500);
    c.success_foreground = Tokens::white();
    c.success_hover = alpha(c.success, 0.9);
    c.success_active = alpha(c.success, 0.8);
    c.warning = tw(Tw::Orange, 300);
    c.warning_foreground = hsla(0.0, 0.0, 0.0, 1.0);
    c.warning_hover = tw(Tw::Orange, 400);
    c.warning_active = tw(Tw::Orange, 500);
    c.info = tw(Tw::Blue, 500);
    c.info_foreground = Tokens::white();
    c.info_hover = alpha(c.info, 0.9);
    c.info_active = alpha(c.info, 0.8);
    c.chart_1 = t.chart[0];
    c.chart_2 = t.chart[1];
    c.chart_3 = t.chart[2];
    c.chart_4 = t.chart[3];
    c.chart_5 = t.chart[4];
    c.chart_grid = alpha(t.border, 0.5);
    c.sidebar = t.sidebar;
    c.sidebar_foreground = t.sidebar_foreground;
    c.sidebar_primary = t.sidebar_primary;
    c.sidebar_primary_foreground = t.sidebar_primary_foreground;
    c.sidebar_accent = t.sidebar_accent;
    c.sidebar_accent_foreground = t.sidebar_accent_foreground;
    c.sidebar_border = t.sidebar_border;
    c.list = t.background;
    c.list_hover = alpha(t.accent, 0.5);
    c.list_active = alpha(t.accent, 0.8);
    c.list_active_border = t.primary;
    c.list_even = t.background;
    c.list_head = t.background;
    c.progress_bar = t.primary;
    c.skeleton = t.accent;
    c.scrollbar = alpha(t.background, 0.0);
    c.scrollbar_thumb = hsla(0.0, 0.0, 1.0, 0.2);
    c.scrollbar_thumb_hover = hsla(0.0, 0.0, 1.0, 0.8);
    c.slider_bar = t.primary;
    c.slider_thumb = t.primary_foreground;
    c.switch = alpha(t.input, 0.8);
    c.switch_thumb = t.foreground;
    c.tab = alpha(t.background, 0.0);
    c.tab_active = alpha(t.input, 0.3);
    c.tab_active_foreground = t.foreground;
    c.tab_bar = t.muted;
    c.tab_bar_segmented = t.muted;
    c.tab_foreground = t.muted_foreground;
    c.table = t.background;
    c.table_head = t.background;
    c.table_head_foreground = t.muted_foreground;
    c.table_row_border = t.border;
    c.title_bar = alpha(t.background, 0.95);
    c.title_bar_border = t.border;
    // gpui-kit's dialog dim also covers the clear shadow border around the window on Linux.
    // `dialog::backdrop` draws the dim instead.
    c.overlay = hsla(0.0, 0.0, 0.0, 0.0);
    c.window_border = hsla(0.0, 0.0, 0.0, 0.0);
    c.group_box = t.card;
    c.group_box_foreground = t.card_foreground;
    c.accordion = alpha(t.background, 0.0);
    c.description_list_label = t.muted;
    c.description_list_label_foreground = t.muted_foreground;

    theme.colors = c;
    theme.font_family = SharedString::from(FONT_SANS);
    theme.mono_font_family = SharedString::from(FONT_MONO);
    theme.font_size = px(16.0);
    theme.mono_font_size = px(14.0);
    theme.radius = radius::MD;
    theme.radius_lg = radius::LG;
    theme.shadow = true;
    // scrollbars are always visible, not only while scrolling
    theme.scrollbar_mode = gpui_kit::component::scroll::ScrollbarMode::Always;
}

// ---------------------------------------------------------------------------------------------------------
// Styling helpers (text sizes, monospace font)
// ---------------------------------------------------------------------------------------------------------

/// Text sizes with their paired line heights, named after Tailwind's text size scale.
pub trait TwText: Styled + Sized {
    /// 12 px text on a 16 px line.
    fn text_xs_tw(self) -> Self {
        self.text_size(px(12.0)).line_height(px(16.0))
    }
    /// 14 px text on a 20 px line.
    fn text_sm_tw(self) -> Self {
        self.text_size(px(14.0)).line_height(px(20.0))
    }
    /// 16 px text on a 24 px line.
    fn text_base_tw(self) -> Self {
        self.text_size(px(16.0)).line_height(px(24.0))
    }
    /// 18 px text on a 28 px line.
    fn text_lg_tw(self) -> Self {
        self.text_size(px(18.0)).line_height(px(28.0))
    }
    /// 20 px text on a 28 px line.
    fn text_xl_tw(self) -> Self {
        self.text_size(px(20.0)).line_height(px(28.0))
    }
    /// 24 px text on a 32 px line.
    fn text_2xl_tw(self) -> Self {
        self.text_size(px(24.0)).line_height(px(32.0))
    }
    /// Explicit pixel size, with the line height equal to the size (for example `text_px_tw(11.0)`).
    fn text_px_tw(self, size: f32) -> Self {
        self.text_size(px(size)).line_height(px(size))
    }
    /// Geist Mono.
    fn font_mono_tw(self) -> Self {
        self.font_family(FONT_MONO)
    }
}

impl<T: Styled + Sized> TwText for T {}

/// Spacing in units of 4 px: `sp(2.5)` is 10 px.
pub fn sp(units: f32) -> Pixels {
    px(units * 4.0)
}

/// 1rem in px at the root font size used by this app.
pub fn rem(n: f32) -> gpui_kit::Rems {
    rems(n)
}

// ---------------------------------------------------------------------------------------------------------
// Shadows
// ---------------------------------------------------------------------------------------------------------

/// One shadow layer straight below the box: `y` px down, `blur` px blur, grown by `spread` px (shrunk when negative).
pub fn box_shadow(color: Hsla, y: f32, blur: f32, spread: f32) -> BoxShadow {
    BoxShadow { color, offset: point(px(0.0), px(y)), blur_radius: px(blur), spread_radius: px(spread), inset: false }
}

fn black(a: f32) -> Hsla {
    hsla(0.0, 0.0, 0.0, a)
}

/// Extra small shadow of inputs and outline buttons: 1 px down, 2 px blur, black at 5% opacity.
pub fn shadow_xs() -> Vec<BoxShadow> {
    vec![box_shadow(black(0.05), 1.0, 2.0, 0.0)]
}

/// Small shadow of cards and badges: two layers 1 px down with 3 px and 2 px blur, black at 10% opacity.
pub fn shadow_sm() -> Vec<BoxShadow> {
    vec![box_shadow(black(0.1), 1.0, 3.0, 0.0), box_shadow(black(0.1), 1.0, 2.0, -1.0)]
}

/// Medium shadow of popup panels: two layers 4 px and 2 px down, black at 10% opacity.
pub fn shadow_md() -> Vec<BoxShadow> {
    shadow_md_tinted(black(0.1))
}

/// `shadow_md` in `color`, for solid buttons whose shadow takes their own colour.
pub fn shadow_md_tinted(color: Hsla) -> Vec<BoxShadow> {
    vec![box_shadow(color, 4.0, 6.0, -1.0), box_shadow(color, 2.0, 4.0, -2.0)]
}

/// Large shadow in `color`: 10 px down, 15 px blur, shrunk by 3 px. Lifts primary buttons and icons in the dialogs.
pub fn shadow_lg_tinted(color: Hsla) -> Vec<BoxShadow> {
    vec![box_shadow(color, 10.0, 15.0, -3.0)]
}

/// Large soft shadow of dialog cards: 25 px down, 50 px blur, shrunk by 12 px, black at 20% opacity.
pub fn shadow_2xl() -> Vec<BoxShadow> {
    vec![box_shadow(black(0.2), 25.0, 50.0, -12.0)]
}

// ---------------------------------------------------------------------------------------------------------
// Colour mixing
// ---------------------------------------------------------------------------------------------------------

/// The value `t` of the way from `a` to `b`: 0 is `a`, 1 is `b`.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The colour `t` of the way from `a` to `b`, mixed per sRGB channel with alpha.
pub fn lerp_color(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b) = (Rgba::from(a), Rgba::from(b));
    Rgba { r: lerp(a.r, b.r, t), g: lerp(a.g, b.g, t), b: lerp(a.b, b.b, t), a: lerp(a.a, b.a, t) }.into()
}

/// `color` at `opacity` composited over `base`: the opaque colour a translucent `color` looks like on top of `base`.
pub fn blend(color: Hsla, opacity: f32, base: Hsla) -> Hsla {
    alpha(lerp_color(base, color, opacity), 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(c: Hsla) -> [f32; 4] {
        let c = Rgba::from(c);
        [c.r, c.g, c.b, c.a].map(|v| (v * 1000.0).round() / 1000.0)
    }

    #[test]
    fn colour_mixing() {
        let red: Hsla = Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }.into();
        let blue: Hsla = Rgba { r: 0.0, g: 0.0, b: 1.0, a: 0.5 }.into();
        assert_eq!(lerp(2.0, 4.0, 0.25), 2.5);
        assert_eq!(rgba(lerp_color(red, blue, 0.0)), rgba(red));
        assert_eq!(rgba(lerp_color(red, blue, 0.5)), [0.5, 0.0, 0.5, 0.75]);
        // blend ignores the alpha of both colours and returns an opaque colour
        assert_eq!(rgba(blend(blue, 0.25, red)), [0.75, 0.0, 0.25, 1.0]);
    }
}
