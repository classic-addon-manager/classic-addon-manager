//! Sidebar: navigation items, update banner, account area and version footer.
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_kit::component::{IconNamed, animation::cubic_bezier};
use gpui_kit::{
    AnyView, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px, svg,
};

use super::{
    icons::{self, Lucide},
    page_registry::{PAGE_DEFINITIONS, PageId},
    theme::{TokensExt, Tw, TwText, alpha, lerp, lerp_color, radius, tw},
};

type ClickFn = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
/// Navigates to the page the user picked.
pub type PageHandler = Rc<dyn Fn(PageId, &mut Window, &mut App)>;
/// Opens the app update dialog from the sidebar banner.
pub type OpenUpdateHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A sidebar entry: icon and label, in the primary colour when active or hovered, with a count badge when
/// `badge_count > 0`.
#[derive(IntoElement)]
pub struct SidebarItem {
    id: SharedString,
    name: SharedString,
    icon: Lucide,
    active: bool,
    badge_count: i64,
    on_click: ClickFn,
}

impl SidebarItem {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        icon: Lucide,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self { id: id.into(), name: name.into(), icon, active: false, badge_count: -1, on_click: Box::new(on_click) }
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    pub fn badge_count(mut self, n: i64) -> Self {
        self.badge_count = n;
        self
    }
}

/// Eased animation of one value from `from` to `target` over `dur`. It can be given a new target while it is running.
#[derive(Clone, Copy)]
struct Track {
    from: f32,
    target: f32,
    start: Instant,
    dur: Duration,
}

impl Track {
    fn value(&self, now: Instant) -> f32 {
        let p = (now.saturating_duration_since(self.start).as_secs_f32() / self.dur.as_secs_f32()).clamp(0.0, 1.0);
        // Ease-in-out with control points (0.4, 0) and (0.2, 1): slow start, fast middle, slow end.
        lerp(self.from, self.target, cubic_bezier(0.4, 0.0, 0.2, 1.0)(p))
    }

    fn running(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) < self.dur
    }

    /// Starts a new animation from the current value when `target` changed, so the value never jumps.
    fn retarget(&mut self, target: f32, now: Instant) {
        if self.target != target {
            *self = Track { from: self.value(now), target, start: now, dur: self.dur };
        }
    }
}

/// Per-item animation state, persisted across frames by element id.
struct ItemAnim {
    hovered: bool,
    /// Opacity of the highlight behind the item, animated over 500 ms: 1 active, 0.1 hovered, 0 idle.
    bg: Cell<Track>,
    /// Mix of text and icon colour from muted-foreground to primary, animated over 300 ms: 1 active or hovered.
    fg: Cell<Track>,
}

impl ItemAnim {
    fn new(active: bool) -> Self {
        let now = Instant::now();
        let track = |target: f32, ms: u64| Track { from: target, target, start: now, dur: Duration::from_millis(ms) };
        Self {
            hovered: false,
            bg: Cell::new(track(if active { 1.0 } else { 0.0 }, 500)),
            fg: Cell::new(track(if active { 1.0 } else { 0.0 }, 300)),
        }
    }
}

impl RenderOnce for SidebarItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let key = SharedString::from(format!("sidebar-{}", self.id));
        let active = self.active;
        let anim = window.use_keyed_state(SharedString::from(format!("sidebar-anim-{}", self.id)), cx, move |_, _| {
            ItemAnim::new(active)
        });

        let now = Instant::now();
        let (bg_opacity, mix) = {
            let a = anim.read(cx);
            let (mut bg, mut fg) = (a.bg.get(), a.fg.get());
            bg.retarget(
                if active {
                    1.0
                } else if a.hovered {
                    0.1
                } else {
                    0.0
                },
                now,
            );
            fg.retarget(if active || a.hovered { 1.0 } else { 0.0 }, now);
            a.bg.set(bg);
            a.fg.set(fg);
            if bg.running(now) || fg.running(now) {
                window.request_animation_frame();
            }
            (bg.value(now), fg.value(now))
        };
        let color = lerp_color(t.muted_foreground, t.primary, mix);

        div()
            .id(key)
            .relative()
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .rounded(radius::LG)
            .px(px(12.0))
            .py(px(8.0))
            .cursor_pointer()
            .text_sm_tw()
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .bg(alpha(t.muted, bg_opacity))
            .on_hover(move |hovered, _, cx| {
                anim.update(cx, |a, cx| {
                    if a.hovered != *hovered {
                        a.hovered = *hovered;
                        cx.notify();
                    }
                });
            })
            .on_click(self.on_click)
            .child(svg().path(self.icon.path()).size(px(16.0)).flex_none().text_color(color))
            .child(div().flex_1().text_color(color).child(self.name))
            .when(self.badge_count > 0, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .rounded_full()
                        .bg(t.primary)
                        .text_color(t.primary_foreground)
                        .text_xs_tw()
                        .child(self.badge_count.to_string()),
                )
            })
    }
}

/// Inputs the shell passes to the sidebar each frame.
pub struct SidebarProps {
    pub active: PageId,
    pub authenticated: bool,
    pub updates_available_count: usize,
    pub update_available: bool,
    pub version: String,
    pub user_bar: AnyView,
    pub on_select: PageHandler,
    pub on_open_update: OpenUpdateHandler,
}

pub fn sidebar(props: SidebarProps, cx: &App) -> impl IntoElement {
    let t = cx.tokens();
    let emerald = tw(Tw::Emerald, 500);
    let emerald_400 = tw(Tw::Emerald, 400);
    let version = props.version.clone();

    let nav = PAGE_DEFINITIONS
        .iter()
        .filter(|p| p.id != PageId::Developer || props.authenticated)
        .map(|p| {
            let id = p.id;
            let on_select = props.on_select.clone();
            SidebarItem::new(format!("{:?}", p.id).to_lowercase(), p.name, p.icon, move |_, window, cx| {
                on_select(id, window, cx)
            })
            .active(props.active == p.id)
            .badge_count(if p.id == PageId::Dashboard { props.updates_available_count as i64 } else { -1 })
            .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(220.0))
        .h_full()
        .min_h_0()
        .border_r_1()
        .border_color(t.border)
        .bg(alpha(t.muted, 0.4))
        .py(px(8.0))
        // nav
        .child(div().flex_1().relative().child(div().flex().flex_col().gap(px(4.0)).px(px(8.0)).children(nav)))
        // update available banner
        .when(props.update_available, |d| {
            let on_open = props.on_open_update.clone();
            d.child(
                div()
                    .id("update-banner")
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .mx(px(8.0))
                    .mb(px(4.0))
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(radius::LG)
                    .border_1()
                    .border_color(alpha(emerald, 0.2))
                    .bg(alpha(emerald, 0.1))
                    .text_color(emerald_400)
                    .text_xs_tw()
                    .cursor_pointer()
                    .hover(move |s| s.bg(alpha(emerald, 0.2)))
                    .on_click(move |_, window, cx| on_open(window, cx))
                    .child(svg().path(icons::CIRCLE_ARROW_UP.path()).size(px(14.0)).flex_none().text_color(emerald_400))
                    .child(div().overflow_hidden().text_ellipsis().whitespace_nowrap().child("Update Available")),
            )
        })
        // account area (`shell::user_bar`)
        .child(div().w_full().child(props.user_bar))
        // version footer
        .child(
            div().mx(px(12.0)).mb(px(12.0)).mt(px(2.0)).child(
                div()
                    .id("version-footer")
                    .group("version-footer")
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(radius::LG)
                    .text_px_tw(11.0)
                    .text_color(alpha(t.muted_foreground, 0.6))
                    .cursor_pointer()
                    .hover(move |s| s.bg(alpha(t.secondary, 0.4)).text_color(t.muted_foreground))
                    .on_click({
                        let version = version.clone();
                        move |_, _, cx| {
                            cx.open_url(&format!(
                                "https://github.com/classic-addon-manager/classic-addon-manager/releases/tag/v{version}"
                            ))
                        }
                    })
                    .child(
                        div()
                            .font_mono_tw()
                            .text_color(alpha(t.muted_foreground, 0.8))
                            .group_hover("version-footer", move |s| s.text_color(alpha(t.foreground, 0.8)))
                            .child(format!("v{version}")),
                    )
                    .child(div().w(px(1.0)).h(px(10.0)).flex_none().bg(alpha(t.border, 0.7)))
                    .child(div().child("made with ❤️ by Sami")),
            ),
        )
}
