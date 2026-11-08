//! One catalog entry, drawn as a row of the list view or as a card of the grid view.
use std::{rc::Rc, time::Duration};

use gpui_kit::component::Icon;
use gpui_kit::{
    App, ElementId, FontWeight, HighlightStyle, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled, StyledText, Task, Window, div, prelude::FluentBuilder as _,
    px, relative,
};

use super::stats::{WarningStyle, action_button, download_meter, like_ring, tag_dots, warning_icon};
use crate::{
    backend::{
        config::{AddonSort, AddonSortKey, AddonViewMode},
        shared::AddonManifest,
    },
    state::queries,
    ui::{
        components::{AddonIcon, addon_icon_url},
        icons,
        theme::{Tokens, TokensExt, TwText, alpha, radius},
        utils::is_new_addon,
    },
};

/// Opens the addon dialog for a manifest.
pub type OpenHandler = Rc<dyn Fn(AddonManifest, &mut Window, &mut App)>;

/// Wait briefly so sweeping the pointer down the list does not prefetch every row it crosses.
const PREFETCH_DELAY: Duration = Duration::from_millis(150);

/// List column widths: icon, name (takes the remaining width), tags, downloads, liked, action.
pub const COL_ICON: f32 = 40.0;
pub const COL_TAGS: f32 = 176.0;
pub const COL_DOWNLOADS: f32 = 76.0;
pub const COL_LIKED: f32 = 56.0;
pub const COL_ACTION: f32 = 92.0;
pub const COL_GAP: f32 = 14.0;

struct PrefetchTimer(Option<Task<()>>);

#[derive(IntoElement)]
pub struct RemoteAddon {
    manifest: AddonManifest,
    installed: bool,
    max_downloads: i64,
    variant: AddonViewMode,
    on_open: OpenHandler,
}

impl RemoteAddon {
    pub fn new(
        manifest: AddonManifest,
        installed: bool,
        max_downloads: i64,
        variant: AddonViewMode,
        on_open: OpenHandler,
    ) -> Self {
        Self { manifest, installed, max_downloads, variant, on_open }
    }
}

/// "NEW" pill shown next to addons that were added recently.
fn new_badge(t: &Tokens) -> impl IntoElement {
    div()
        .flex_none()
        .rounded(radius::SM)
        .bg(t.primary)
        .px(px(6.0))
        .py(px(2.0))
        .text_size(px(10.0))
        .line_height(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(t.primary_foreground)
        .child("NEW")
}

/// Icon with the green "installed" tick at its bottom-right corner.
fn icon_with_tick(manifest: &AddonManifest, installed: bool, t: &Tokens) -> impl IntoElement {
    let url = addon_icon_url(manifest.icon.as_deref(), None, None, false);
    div().relative().flex_none().child(AddonIcon::new(&manifest.alias, url)).when(installed, |d| {
        d.child(
            div()
                .absolute()
                .bottom(px(-4.0))
                .right(px(-4.0))
                .flex()
                .items_center()
                .justify_center()
                .size(px(16.0))
                .rounded_full()
                .border_2()
                .border_color(t.background)
                .bg(crate::ui::theme::tw(crate::ui::theme::Tw::Green, 500))
                .text_color(t.background)
                .child(Icon::new(icons::lucide("check-heavy")).size(px(8.0))),
        )
    })
}

/// Appends `text` with every whitespace run (newlines included) collapsed to a single space, so multi-line
/// descriptions flow as one paragraph instead of breaking the row.
fn push_collapsed(out: &mut String, text: &str) {
    for (i, word) in text.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(word);
    }
}

/// `author · description` on one line that is clipped without an ellipsis. The author is drawn in the muted text colour
/// and the description in a fainter shade of it.
fn author_line(manifest: &AddonManifest, t: &Tokens) -> impl IntoElement {
    let author = manifest.author.clone();
    let mut text = author.clone();
    if !manifest.description.trim().is_empty() {
        text.push_str(" · ");
        push_collapsed(&mut text, &manifest.description);
    }
    let split = author.len();
    let muted = t.muted_foreground;
    let highlights = vec![(0..split, HighlightStyle { color: Some(muted), ..Default::default() })];
    div()
        .mt(px(2.0))
        .overflow_hidden()
        .whitespace_nowrap()
        .text_xs_tw()
        .text_color(alpha(muted, 0.7))
        .child(StyledText::new(text).with_highlights(highlights))
}

impl RenderOnce for RemoteAddon {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let manifest = self.manifest;
        let group: SharedString = format!("addon-item-{}", manifest.name).into();
        let id: ElementId = ElementId::Name(group.clone());
        let is_new = is_new_addon(manifest.added_at);

        let prefetch = window.use_keyed_state((id.clone(), "prefetch"), cx, |_, _| PrefetchTimer(None));
        // `None` until first hovered, so the action button renders statically instead of animating a collapse.
        let hover = window.use_keyed_state((id.clone(), "hover"), cx, |_, _| None::<bool>);
        let hovered = *hover.read(cx);
        let prefetch_manifest = manifest.clone();
        let on_hover = move |hovered: &bool, _: &mut Window, cx: &mut App| {
            hover.update(cx, |h, cx| {
                *h = Some(*hovered);
                cx.notify();
            });
            if *hovered {
                let manifest = prefetch_manifest.clone();
                let task = cx.spawn(async move |cx| {
                    cx.background_executor().timer(PREFETCH_DELAY).await;
                    cx.update(|cx| queries::prefetch_addon_details(cx, &manifest));
                });
                prefetch.update(cx, |p, _| p.0 = Some(task));
            } else {
                prefetch.update(cx, |p, _| p.0 = None);
            }
        };

        let on_open = self.on_open;
        let open_manifest = manifest.clone();
        let on_click = move |_: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut App| {
            on_open(open_manifest.clone(), window, cx)
        };

        let primary = t.primary;
        let installed = self.installed;
        match self.variant {
            AddonViewMode::List => {
                let alias = div()
                    .truncate()
                    .text_base_tw()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if hovered == Some(true) { primary } else { t.foreground })
                    .child(manifest.alias.clone());
                div()
                    .id(id.clone())
                    .group(group.clone())
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(COL_GAP))
                    .w_full()
                    .px(px(12.0))
                    .py(px(10.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(alpha(t.muted, 0.5)))
                    .on_hover(on_hover)
                    .on_click(on_click)
                    // Accent bar at the left edge that shows while the row is hovered
                    .child(
                        div()
                            .absolute()
                            .top(px(10.0))
                            .bottom(px(10.0))
                            .left_0()
                            .w(px(2.0))
                            .rounded_full()
                            .group_hover(group.clone(), move |s| s.bg(primary)),
                    )
                    .child(icon_with_tick(&manifest, installed, &t))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .min_w_0()
                                    .child(alias)
                                    .when(is_new, |d| d.child(new_badge(&t)))
                                    .children(warning_icon(
                                        manifest.warning.as_deref(),
                                        WarningStyle::Chip,
                                        (id.clone(), "warning").into(),
                                    )),
                            )
                            .child(author_line(&manifest, &t)),
                    )
                    .child(div().flex_none().w(px(COL_TAGS)).min_w_0().overflow_hidden().children(tag_dots(
                        &manifest.tags,
                        3,
                        &t,
                    )))
                    .child(div().flex_none().w(px(COL_DOWNLOADS)).child(download_meter(
                        manifest.downloads,
                        self.max_downloads,
                        &t,
                    )))
                    .child(div().flex_none().w(px(COL_LIKED)).child(like_ring(
                        manifest.like_percent.map(i64::from),
                        true,
                        &t,
                    )))
                    .child(div().flex().flex_none().justify_end().w(px(COL_ACTION)).child(action_button(
                        installed,
                        false,
                        hovered,
                        (id.clone(), "action").into(),
                        window,
                        &t,
                    )))
                    .into_any_element()
            }
            AddonViewMode::Grid => {
                let alias = div()
                    .truncate()
                    .text_sm_tw()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if hovered == Some(true) { primary } else { t.foreground })
                    .child(manifest.alias.clone());
                let border = alpha(t.border, 0.6);
                div()
                    .id(id.clone())
                    .group(group.clone())
                    .flex()
                    .flex_col()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .rounded(radius::XL)
                    .border_1()
                    .border_color(border)
                    .bg(alpha(t.muted, 0.2))
                    .cursor_pointer()
                    .hover(|s| s.border_color(alpha(primary, 0.35)).bg(alpha(t.muted, 0.5)))
                    .on_hover(on_hover)
                    .on_click(on_click)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .min_w_0()
                            .px(px(12.0))
                            .pt(px(12.0))
                            .child(icon_with_tick(&manifest, installed, &t))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .min_w_0()
                                            .child(alias)
                                            .when(is_new, |d| d.child(new_badge(&t)))
                                            .children(warning_icon(
                                                manifest.warning.as_deref(),
                                                WarningStyle::Compact,
                                                (id.clone(), "warning").into(),
                                            )),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_xs_tw()
                                            .text_color(t.muted_foreground)
                                            .child(manifest.author.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div().mt(px(10.0)).min_h(px(32.0)).px(px(12.0)).child(
                            div()
                                .overflow_hidden()
                                .line_clamp(2)
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(alpha(t.muted_foreground, 0.7))
                                .child({
                                    let mut description = String::with_capacity(manifest.description.len());
                                    push_collapsed(&mut description, &manifest.description);
                                    description
                                }),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .min_h(px(20.0))
                            .mt(px(8.0))
                            .mb(px(10.0))
                            .px(px(12.0))
                            .children(tag_dots(&manifest.tags, 3, &t)),
                    )
                    .child(
                        div()
                            .mt_auto()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .h(px(40.0))
                            .px(px(12.0))
                            .border_t_1()
                            .border_color(border)
                            .child(download_meter(manifest.downloads, self.max_downloads, &t))
                            .child(div().flex_none().child(like_ring(manifest.like_percent.map(i64::from), false, &t)))
                            // A flexible spacer pushes the action button to the right. Pushing with `ml_auto` left
                            // about 24px of the free space to the right of the button and swallowed the `gap` between
                            // the cells.
                            .child(div().flex_1().min_w_0())
                            .child(div().flex().flex_none().whitespace_nowrap().child(action_button(
                                installed,
                                true,
                                hovered,
                                (id.clone(), "action").into(),
                                window,
                                &t,
                            ))),
                    )
                    .into_any_element()
            }
        }
    }
}

/// The column captions above the list rows. "Addon", "Downloads" and "Liked" sort the catalog: the active
/// one is highlighted with its direction arrow and flips direction when clicked again. Nothing moves when the order
/// changes: the weight stays the same (bold widens the glyphs) and the arrow hangs into the column gap after the
/// label (COL_GAP fits it), so the right-aligned labels keep their place.
pub fn list_header(sort: AddonSort, t: &Tokens) -> impl IntoElement {
    let muted = alpha(t.muted_foreground, 0.7);
    let (primary, foreground) = (t.primary, t.foreground);
    let cap = |label: &'static str| {
        div().text_size(px(10.0)).line_height(px(15.0)).text_color(muted).child(label.to_uppercase())
    };
    let sortable = |id: &'static str, label: &'static str, key: AddonSortKey| {
        let active = sort.key() == key;
        let color = if active { primary } else { muted };
        let arrow = if sort.descending() { icons::ARROW_DOWN } else { icons::ARROW_UP };
        // The arrow lies outside the label's hitbox, so it takes the click too.
        let flip = move |_: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut App| {
            crate::state::preferences(cx).update(cx, |p, cx| p.set_addon_sort(sort.clicked(key), cx));
        };
        div()
            .id(id)
            .relative()
            .cursor_pointer()
            .text_size(px(10.0))
            .line_height(px(15.0))
            .text_color(color)
            .when(!active, |d| d.hover(move |s| s.text_color(foreground)))
            .on_click(flip)
            .child(label.to_uppercase())
            .when(active, |d| {
                d.child(
                    div()
                        .id((id, 1usize))
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(1.0))
                        .pl(px(3.0))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .on_click(flip)
                        .child(Icon::new(arrow).size(px(10.0)).text_color(color)),
                )
            })
    };
    div()
        .flex()
        .items_center()
        .gap(px(COL_GAP))
        .px(px(12.0))
        .pb(px(4.0))
        .child(div().flex_none().w(px(COL_ICON)))
        .child(div().flex().flex_1().min_w_0().child(sortable("addons-sort-name", "Addon", AddonSortKey::Name)))
        .child(cap("Tags").flex_none().w(px(COL_TAGS)))
        .child(div().flex().flex_none().w(px(COL_DOWNLOADS)).justify_end().child(sortable(
            "addons-sort-downloads",
            "Downloads",
            AddonSortKey::Downloads,
        )))
        .child(div().flex().flex_none().w(px(COL_LIKED)).justify_end().child(sortable(
            "addons-sort-rating",
            "Liked",
            AddonSortKey::Rating,
        )))
        .child(div().flex_none().w(px(COL_ACTION)))
}
