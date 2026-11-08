//! Toolbar of the addons page: search box, tag select, sort select, list/grid toggle and refresh button. It is shown in
//! the title bar.
use std::time::Instant;

use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled, Window, div,
    px,
};

use super::catalog::CatalogState;
use crate::{
    backend::config::{AddonSort, AddonSortKey, AddonViewMode},
    state,
    ui::{
        components::{
            Button, ButtonSize, ButtonVariant, Input, InputState, Select, SelectItem, SelectSize, Side, ToggleGroup,
            ToggleGroupItem, ToggleSize, ToggleVariant, Tooltip, on_input_settled, spinner, text_input_with_value,
        },
        icons,
        titlebar::NoDrag as _,
        toast::{self, Toast},
        utils::truncate_chars,
    },
};

/// Value of the list/grid toggle item for `mode`.
fn view_mode_value(mode: AddonViewMode) -> &'static str {
    match mode {
        AddonViewMode::List => "list",
        AddonViewMode::Grid => "grid",
    }
}

/// Text of the sort menu row for `sort`. It is also the row's value, since every order has its own text.
fn sort_label(sort: AddonSort) -> &'static str {
    match sort {
        AddonSort::Default => "New first",
        AddonSort::NameAsc => "Name (A–Z)",
        AddonSort::NameDesc => "Name (Z–A)",
        AddonSort::DownloadsDesc => "Most downloaded",
        AddonSort::DownloadsAsc => "Least downloaded",
        AddonSort::RatingDesc => "Highest rated",
        AddonSort::RatingAsc => "Lowest rated",
    }
}

/// Symbol of a sort key, used in the trigger and on the menu rows.
fn sort_icon(key: AddonSortKey) -> icons::Lucide {
    match key {
        AddonSortKey::Default => icons::CLOCK,
        AddonSortKey::Name => icons::CASE_SENSITIVE,
        AddonSortKey::Downloads => icons::DOWNLOAD,
        AddonSortKey::Rating => icons::HEART,
    }
}

/// Icon-only sort trigger (the title bar is tight at the default window size): the key's symbol plus a direction
/// arrow, the full wording is in the tooltip and the menu. "New first" has no direction but keeps the arrow's slot,
/// so the trigger, and the toolbar items left of it, keep their width across orders.
fn sort_trigger(sort: AddonSort) -> impl IntoElement {
    let arrow = (sort.key() != AddonSortKey::Default)
        .then(|| Icon::new(if sort.descending() { icons::ARROW_DOWN } else { icons::ARROW_UP }).size(px(12.0)));
    let arrow = div().flex_none().size(px(12.0)).children(arrow);
    div().flex().items_center().gap(px(2.0)).child(Icon::new(sort_icon(sort.key())).size(px(16.0))).child(arrow)
}

pub struct AddonsToolbar {
    catalog: Entity<CatalogState>,
    search: Entity<InputState>,
}

impl AddonsToolbar {
    pub fn new(catalog: Entity<CatalogState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The search text lives in the shared catalog state, which outlives the page, so the box shows the active filter
        // again when you come back.
        let initial = catalog.read(cx).search_query.clone();
        let search = text_input_with_value(window, cx, "Search addons...", initial);
        on_input_settled(&search, window, cx, |this: &mut Self, text, cx| {
            this.catalog.update(cx, |c, cx| {
                if c.search_query != text {
                    c.search_query = text;
                    cx.notify();
                }
            });
        });
        cx.observe(&catalog, |_, _, cx| cx.notify()).detach();
        cx.observe(&state::preferences(cx), |_, _, cx| cx.notify()).detach();
        Self { catalog, search }
    }

    /// Fetches the catalog from the server, keeping the spinner up for at least [`state::tasks::MIN_SPINNER`], and
    /// toasts the result.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.catalog.read(cx).is_refreshing {
            return;
        }
        let started = Instant::now();
        self.catalog.update(cx, |c, cx| {
            c.is_refreshing = true;
            cx.notify();
        });
        let refresh = state::catalog::refresh(cx);
        let catalog = self.catalog.clone();
        cx.spawn(async move |_, cx| {
            let result = refresh.await;
            let elapsed = started.elapsed();
            state::hold_spinner(started, cx).await;
            cx.update(|cx| {
                catalog.update(cx, |c, cx| {
                    c.is_refreshing = false;
                    cx.notify();
                });
                let toast = match result {
                    Err(e) => {
                        let message = format!("Failed to refresh addons: {}", truncate_chars(&e.message(), 100));
                        Toast::new("Error", message).icon(icons::TRIANGLE_ALERT)
                    }
                    Ok(_) => Toast::new("Completed", format!("Refreshed addons in {}ms", elapsed.as_millis()))
                        .icon(icons::REFRESH_CW),
                };
                toast::show(cx, toast);
            });
        })
        .detach();
    }
}

impl Render for AddonsToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let catalog = self.catalog.read(cx);
        let ready = catalog.is_ready;
        let refreshing = catalog.is_refreshing;
        let selected_tag = catalog.selected_tag.clone();
        let tags: Vec<(String, String)> = catalog.tags.iter().map(|t| (t.clone(), t.clone())).collect();
        let prefs = state::preferences(cx).read(cx);
        let (view_mode, sort) = (prefs.addon_view_mode, prefs.addon_sort);
        let catalog_entity = self.catalog.clone();

        let refresh_icon = if refreshing {
            div().mr(px(6.0)).child(spinner().with_size(px(14.0))).into_any_element()
        } else {
            Icon::new(icons::REFRESH_CW).size(px(14.0)).mr(px(6.0)).into_any_element()
        };

        div()
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .no_drag()
                    .relative()
                    .min_w_0()
                    .flex_1()
                    .child(Input::search(&self.search).cleanable(true).disabled(!ready).w_full()),
            )
            .child(
                div().flex().flex_none().items_center().gap(px(8.0)).child(
                    div()
                        .no_drag()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            Select::new("addons-tag")
                                .size(SelectSize::Sm)
                                .width(px(140.0))
                                .items(tags)
                                .value(selected_tag.clone())
                                .trigger_label(selected_tag)
                                .disabled(!ready)
                                .on_change(move |value, _window, cx| {
                                    let value = value.to_string();
                                    catalog_entity.update(cx, |c, cx| {
                                        c.selected_tag = value;
                                        cx.notify();
                                    });
                                }),
                        )
                        .child(
                            Tooltip::new(format!("Sort: {}", sort_label(sort))).side(Side::Bottom).child(
                                AddonSort::ALL
                                    .into_iter()
                                    .fold(
                                        Select::new("addons-sort").size(SelectSize::Sm).label("Sort by"),
                                        |select, s| {
                                            select.item(
                                                SelectItem::new(sort_label(s), sort_label(s)).icon(sort_icon(s.key())),
                                            )
                                        },
                                    )
                                    .value(sort_label(sort))
                                    .trigger_content(sort_trigger(sort))
                                    .disabled(!ready)
                                    .on_change(|value, _window, cx| {
                                        if let Some(sort) = AddonSort::ALL.into_iter().find(|&s| value == sort_label(s))
                                        {
                                            state::preferences(cx).update(cx, |p, cx| p.set_addon_sort(sort, cx));
                                        }
                                    }),
                            ),
                        )
                        .child(
                            ToggleGroup::new("addons-view-mode")
                                .variant(ToggleVariant::Outline)
                                .size(ToggleSize::Sm)
                                .value(view_mode_value(view_mode))
                                .disabled(!ready)
                                .on_value_change(|value, _window, cx| {
                                    let mode = if value == view_mode_value(AddonViewMode::Grid) {
                                        AddonViewMode::Grid
                                    } else {
                                        AddonViewMode::List
                                    };
                                    state::preferences(cx).update(cx, |p, cx| p.set_addon_view_mode(mode, cx));
                                })
                                .item(ToggleGroupItem::new(view_mode_value(AddonViewMode::List)).icon(icons::LIST))
                                .item(
                                    ToggleGroupItem::new(view_mode_value(AddonViewMode::Grid)).icon(icons::LAYOUT_GRID),
                                ),
                        )
                        .child(
                            Button::new("addons-refresh")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .has_icon(true)
                                .disabled(!ready || refreshing)
                                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                                .child(refresh_icon)
                                .child("Refresh"),
                        ),
                ),
            )
    }
}
