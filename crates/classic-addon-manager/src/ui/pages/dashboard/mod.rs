//! Dashboard page: installed addon list, details pane of the selected addon, update and version dialogs, and the title
//! bar toolbar.
mod details;
mod install_flow;
mod install_zip;
mod list_item;
mod model;
mod phase_ui;
mod repo_match;
mod toolbar;
mod update_dialog;
mod version_dialog;

use std::{
    collections::{HashMap, HashSet},
    mem, ptr,
    sync::{Arc, Weak},
    time::Duration,
};

use cam_core::{addon::Addon, api::Release};
use cam_ui::{
    anim::{fade_in, fade_slide_in},
    components::{Button, ButtonVariant, EmptyState, EmptyTone, ScrollArea, spinner},
    icons,
    theme::{TokensExt, alpha, box_shadow},
    toast,
};
use gpui_kit::component::Sizable as _;
use gpui_kit::{
    Animation, AnimationExt as _, App, AppContext as _, Context, Entity, ExternalPaths, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};

use self::{
    details::DetailsPane,
    list_item::AddonListItem,
    model::{DashboardModel, ListedAddon, filtered_addons, model, reconcile_selection},
};
use crate::{
    state::{self, AddonState},
    ui::{
        pages::PageId,
        shell::{self, titlebar},
    },
};

pub struct DashboardPage {
    model: Entity<DashboardModel>,
    addons: Entity<AddonState>,
    /// True until the first reload of the installed addons has finished.
    is_loading: bool,
    details: Option<Entity<DetailsPane>>,
    /// The list as drawn last. Rows that appear later fade in.
    order: ListOrder,
}

/// The listed addons in list order, computed from the snapshots and search text it holds. Kept until one of them
/// changes, so a draw that changes none of them reuses it.
#[derive(Default)]
struct ListOrder {
    installed: Arc<Vec<Addon>>,
    /// Only tells whether the releases changed. Weak, so it does not keep replaced release notes alive; the
    /// allocation it points to stays reserved, so its address can't be reused by a later map.
    releases: Weak<HashMap<String, Release>>,
    query: String,
    rows: Vec<Row>,
}

struct Row {
    listed: ListedAddon,
    /// Element id of the row.
    id: SharedString,
}

impl ListOrder {
    fn new(installed: Arc<Vec<Addon>>, releases: Arc<HashMap<String, Release>>, query: &str) -> Self {
        let rows = filtered_addons(&installed, &releases, query)
            .into_iter()
            .map(|listed| Row { listed, id: SharedString::from(format!("addon-{}", installed[listed.index].name)) })
            .collect();
        Self { installed, releases: Arc::downgrade(&releases), query: query.to_owned(), rows }
    }

    fn is_for(&self, installed: &Arc<Vec<Addon>>, releases: &Arc<HashMap<String, Release>>, query: &str) -> bool {
        Arc::ptr_eq(&self.installed, installed)
            && ptr::eq(self.releases.as_ptr(), Arc::as_ptr(releases))
            && self.query == query
    }

    /// True when both list the same addons, perhaps in another order: which addons are listed follows from the
    /// installed list and the search text alone.
    fn lists_same_addons(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.installed, &other.installed) && self.query == other.query
    }

    fn addon(&self, row: &Row) -> &Addon {
        &self.installed[row.listed.index]
    }

    /// The listed addons in list order.
    fn addons(&self) -> impl Iterator<Item = &Addon> {
        self.rows.iter().map(|row| self.addon(row))
    }
}

impl DashboardPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = model(cx);
        let addons = state::addons(cx);
        model.update(cx, |m, _| m.search_query.clear());

        cx.observe(&addons, |_, _, cx| cx.notify()).detach();
        cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()).detach();
        cx.observe(&model, |_, _, cx| cx.notify()).detach();

        let toolbar = toolbar::new_toolbar(model.clone(), window, cx);

        // Load the installed addons, then check them all for updates. The catalog (for the icons) loads alongside.
        let refresh = addons.update(cx, |s, cx| s.refresh(cx));
        cx.spawn(async move |this, cx| {
            refresh.await;
            this.update(cx, |this, cx| {
                this.is_loading = false;
                // The toolbar is only shown once the list is: it stays out of the title bar while loading.
                titlebar::set_slot(cx, toolbar);
                cx.notify();
            })
        })
        .detach();
        state::catalog::ensure(cx);

        Self { model, addons, is_loading: true, details: None, order: ListOrder::default() }
    }

    fn loading_view(&self, cx: &App) -> impl IntoElement {
        let t = cx.tokens();
        let primary = t.primary;
        let pulse = |el: gpui_kit::Div| {
            el.with_animation(
                "loading-pulse",
                Animation::new(Duration::from_secs(2)).repeat().with_easing(gpui_kit::pulsating_between(0.5, 1.0)),
                |el, v| el.opacity(v),
            )
        };
        div()
            .flex()
            .h_full()
            .min_h_0()
            .items_center()
            .justify_center()
            .bg(t.background)
            .child(
                div()
                    .relative()
                    .size(px(48.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    // Large soft glow behind the spinner, pulsing
                    .child(pulse(
                        div()
                            .absolute()
                            .left(px(-32.0))
                            .top(px(-32.0))
                            .size(px(112.0))
                            .rounded_full()
                            .bg(alpha(primary, 0.1))
                            .shadow(vec![box_shadow(alpha(primary, 0.1), 0.0, 24.0, 8.0)]),
                    ))
                    // Ring that expands and fades out repeatedly around the spinner
                    .child(div().absolute().left(px(-16.0)).top(px(-16.0)).size(px(80.0)).child(
                        div().size_full().rounded_full().bg(alpha(primary, 0.2)).with_animation(
                            "loading-ping",
                            Animation::new(Duration::from_secs(1)).repeat().with_easing(gpui_kit::ease_out_quint()),
                            |el, v| {
                                let scale = 1.0 + v;
                                el.absolute()
                                    .left(px(40.0 - 40.0 * scale))
                                    .top(px(40.0 - 40.0 * scale))
                                    .size(px(80.0 * scale))
                                    .opacity(0.75 * (1.0 - v))
                            },
                        ),
                    ))
                    .child(div().relative().child(spinner().with_size(px(48.0)).color(primary))),
            )
            .child(div().ml_6().text_color(t.muted_foreground).child("Loading your addons...").with_animation(
                "loading-text",
                Animation::new(Duration::from_secs(2)).repeat().with_easing(gpui_kit::pulsating_between(0.5, 1.0)),
                |el, v| el.opacity(v),
            ))
    }

    fn no_addons_view(&self) -> impl IntoElement {
        EmptyState::new(icons::PACKAGE, "No Addons Installed")
            .tone(EmptyTone::Primary)
            .description("Get started by browsing our collection of addons or install from a ZIP file.")
            .size_full()
            .p_8()
            .child(
                Button::new("browse-addons")
                    .icon(icons::BLOCKS)
                    .label("Browse Addons")
                    .on_click(|_, window, cx| shell::navigate(PageId::Addons, window, cx)),
            )
            .child(
                Button::new("empty-install-zip")
                    .variant(ButtonVariant::Outline)
                    .icon(icons::UPLOAD)
                    .label("Install ZIP")
                    .on_click(|_, _, cx| install_zip::pick_and_install(cx)),
            )
    }
}

fn no_selected_addon() -> impl IntoElement {
    EmptyState::new(icons::BLOCKS, "Select an addon")
        .description("Choose an addon from the list to view its details, actions, and description.")
        .size_full()
        .p_8()
}

impl Render for DashboardPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let drop_target = |el: gpui_kit::Div, cx: &mut Context<Self>| {
            el.on_drop(cx.listener(|_, paths: &ExternalPaths, _window, cx| {
                let zips = install_zip::zip_files(paths.paths());
                if zips.is_empty() {
                    toast::show(
                        cx,
                        toast::Toast::new("Error", "Only ZIP files can be installed").icon(icons::TRIANGLE_ALERT),
                    );
                } else {
                    install_zip::install_paths(zips, cx);
                }
            }))
        };

        if self.is_loading {
            return div().size_full().child(self.loading_view(cx)).into_any_element();
        }

        let (installed, releases, checking) = {
            let store = self.addons.read(cx);
            (store.installed_addons.clone(), store.latest_releases.clone(), store.is_checking_for_updates)
        };

        if installed.is_empty() {
            return drop_target(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .min_h_0()
                    .bg(t.background)
                    .child(div().flex_1().min_h_0().overflow_hidden().child(self.no_addons_view())),
                cx,
            )
            .into_any_element();
        }

        // Reorder only when the installed list, the releases or the search text changed since the last draw
        let query = self.model.read(cx).search_query.as_str();
        let previous = (!self.order.is_for(&installed, &releases, query))
            .then(|| mem::replace(&mut self.order, ListOrder::new(installed, releases, query)));
        // Rows missing from the previous list fade in, unless that list was empty
        let previous_names: Option<HashSet<&str>> = previous
            .as_ref()
            .filter(|p| !p.rows.is_empty() && !p.lists_same_addons(&self.order))
            .map(|p| p.addons().map(|a| a.name.as_str()).collect());
        let order = &self.order;

        // Keep the selection valid for the filtered list. The selected addon is shown as its installed copy.
        let mut replacement = None;
        let selected: Option<&Addon> = {
            let current = self.model.read(cx).selected.as_ref().map(|s| s.name.as_str());
            let name = match reconcile_selection(current, order.addons()) {
                None => current,
                Some(new) => {
                    replacement = Some(new.cloned());
                    new.map(|a| a.name.as_str())
                }
            };
            name.and_then(|name| order.installed.iter().find(|a| a.name == name))
        };
        if let Some(new) = replacement {
            self.model.update(cx, |m, cx| m.set_selected(new, cx));
        }

        // The details view is rebuilt whenever the selected addon's name, repo or branch changes
        match selected {
            Some(addon) => {
                if self.details.as_ref().is_none_or(|pane| !pane.read(cx).shows(addon)) {
                    let (addon, model) = (addon.clone(), self.model.clone());
                    self.details = Some(cx.new(|cx| DetailsPane::new(addon, model, cx)));
                }
            }
            None => self.details = None,
        }

        let catalog = state::catalog::current(cx);
        let selected_name = selected.map(|a| a.name.as_str());
        let items: Vec<gpui_kit::AnyElement> = order
            .rows
            .iter()
            .map(|row| {
                let addon = order.addon(row);
                let (model, installed, index) = (self.model.clone(), order.installed.clone(), row.listed.index);
                let item = AddonListItem::new(
                    row.id.clone(),
                    addon,
                    selected_name == Some(addon.name.as_str()),
                    row.listed.has_update,
                    checking,
                    catalog.as_deref().and_then(|c| c.icon_url(&addon.name)),
                    move |_, cx| model.update(cx, |m, cx| m.set_selected(Some(installed[index].clone()), cx)),
                );
                let wrapper = div().child(item);
                if previous_names.as_ref().is_some_and(|names| !names.contains(addon.name.as_str())) {
                    fade_slide_in(wrapper, SharedString::from(format!("row-in-{}", addon.name))).into_any_element()
                } else {
                    wrapper.into_any_element()
                }
            })
            .collect();

        let list: gpui_kit::AnyElement = if order.rows.is_empty() {
            fade_in(
                EmptyState::new(icons::PACKAGE_SEARCH, "No addons match your search")
                    .description("Try a different query or clear the search.")
                    .compact()
                    .min_h(px(200.0))
                    .p_8(),
                "no-results",
                Duration::from_millis(250),
            )
            .into_any_element()
        } else {
            div().flex().flex_col().gap(px(6.0)).children(items).into_any_element()
        };

        let right: gpui_kit::AnyElement = match &self.details {
            Some(pane) => pane.clone().into_any_element(),
            None => no_selected_addon().into_any_element(),
        };

        drop_target(
            div().flex().flex_col().size_full().min_h_0().bg(t.background).child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_none()
                            .w(px(280.0))
                            .h_full()
                            .border_r_1()
                            .border_color(t.border)
                            .child(
                                ScrollArea::new("addon-list").flex_1().min_h_0().content_style(|d| d.p_2()).child(list),
                            ),
                    )
                    .child(div().flex_1().w_0().h_full().overflow_hidden().child(right)),
            ),
            cx,
        )
        .into_any_element()
    }
}
