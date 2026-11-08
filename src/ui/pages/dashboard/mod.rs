//! Dashboard page: installed addon list, details pane of the selected addon, update and version dialogs, and the title
//! bar toolbar.
mod atoms;
mod details;
mod install_flow;
mod install_zip;
mod list_item;
mod phase_ui;
mod repo_match;
mod toolbar;
mod update_dialog;
mod version_dialog;

use std::{collections::HashSet, time::Duration};

use gpui_kit::component::Sizable as _;
use gpui_kit::{
    Animation, AnimationExt as _, App, AppContext as _, Context, Entity, ExternalPaths, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use self::{
    atoms::{DashboardModel, filtered_addons, model, reconcile_selection, resolve_selected},
    details::DetailsPane,
    list_item::AddonListItem,
    toolbar::DashboardToolbar,
};
use crate::{
    backend::addon,
    state::{self, AddonState},
    ui::{
        anim::{fade_in, fade_slide_in},
        components::{Button, ButtonSize, ButtonVariant, EmptyState, EmptyTone, ScrollArea, spinner},
        icons,
        page_registry::PageId,
        shell,
        theme::{TokensExt, alpha, box_shadow},
        titlebar, toast,
    },
};

pub struct DashboardPage {
    model: Entity<DashboardModel>,
    addons: Entity<AddonState>,
    /// True until the first reload of the installed addons has finished.
    is_loading: bool,
    details: Option<(String, Entity<DetailsPane>)>,
    /// Names rendered in the list so far. Rows that appear later fade in.
    rendered_names: HashSet<String>,
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

        Self { model, addons, is_loading: true, details: None, rendered_names: HashSet::new() }
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

        let store = self.addons.read(cx);
        let installed = store.installed_addons.clone();
        let releases = store.latest_releases.clone();
        let checking = store.is_checking_for_updates;

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

        // Keep the selection valid for the filtered list
        let filtered = filtered_addons(&installed, &releases, &self.model.read(cx).search_query);
        let snapshot = self.model.read(cx).selected.clone();
        let mut selected = resolve_selected(&installed, snapshot.as_ref());
        if let Some(new) = reconcile_selection(selected.as_ref(), &filtered) {
            self.model.update(cx, |m, cx| m.set_selected(new.clone(), cx));
            selected = resolve_selected(&installed, new.as_ref());
        }

        // The details view is rebuilt whenever the selected addon's `name:repo:branch` key changes
        match &selected {
            Some(addon) => {
                let key = DetailsPane::key_of(addon);
                if self.details.as_ref().is_none_or(|(k, _)| *k != key) {
                    let (addon, model) = (addon.clone(), self.model.clone());
                    self.details = Some((key, cx.new(|cx| DetailsPane::new(addon, model, cx))));
                }
            }
            None => self.details = None,
        }

        let catalog = state::catalog::current(cx);
        let animate_new = !self.rendered_names.is_empty();
        let items: Vec<gpui_kit::AnyElement> = filtered
            .iter()
            .map(|addon| {
                let is_new = animate_new && !self.rendered_names.contains(&addon.name);
                let model = self.model.clone();
                let clicked = addon.clone();
                let row = AddonListItem::new(
                    SharedString::from(format!("addon-{}", addon.name)),
                    addon.clone(),
                    selected.as_ref().is_some_and(|s| s.name == addon.name),
                    releases.get(&addon.name).is_some_and(|r| addon::has_update(addon, r)),
                    checking,
                    catalog.as_ref().and_then(|c| c.icon_url(&addon.name)).map(str::to_owned),
                    move |_, cx| model.update(cx, |m, cx| m.set_selected(Some(clicked.clone()), cx)),
                );
                let wrapper = div().child(row);
                if is_new {
                    fade_slide_in(wrapper, SharedString::from(format!("row-in-{}", addon.name))).into_any_element()
                } else {
                    wrapper.into_any_element()
                }
            })
            .collect();
        self.rendered_names = filtered.iter().map(|a| a.name.clone()).collect();

        let list: gpui_kit::AnyElement = if filtered.is_empty() {
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
            Some((_, pane)) => pane.clone().into_any_element(),
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
