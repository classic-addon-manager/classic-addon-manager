//! Toolbar of the dashboard: search box, Refresh and Install ZIP buttons and the addon and update count badges. It is
//! shown in the title bar.
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};

use super::{atoms::DashboardModel, install_zip};
use crate::{
    state::{self, AddonState},
    ui::{
        components::{
            Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Input, InputState, Tooltip, on_input_settled,
            spinner, text_input,
        },
        icons,
        theme::{TokensExt, alpha},
        titlebar::NoDrag as _,
    },
};

pub struct DashboardToolbar {
    search: Entity<InputState>,
    model: Entity<DashboardModel>,
    addons: Entity<AddonState>,
}

impl DashboardToolbar {
    pub fn new(model: Entity<DashboardModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let addons = state::addons(cx);
        cx.observe(&addons, |_, _, cx| cx.notify()).detach();
        cx.observe(&model, |_, _, cx| cx.notify()).detach();
        let search = text_input(window, cx, "Search installed addons");
        on_input_settled(&search, window, cx, |this: &mut Self, query, cx| {
            this.model.update(cx, |m, cx| {
                m.search_query = query;
                cx.notify();
            });
        });
        Self { search, model, addons }
    }
}

impl Render for DashboardToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let store = self.addons.read(cx);
        let (count, updates, checking) =
            (store.installed_addons.len(), store.updates_available_count, store.is_checking_for_updates);
        let installing = store.is_installing_zip();
        let addons = self.addons.clone();

        let refresh_icon = if checking {
            spinner().with_size(px(14.0)).into_any_element()
        } else {
            Icon::new(icons::REFRESH_CW).size(px(14.0)).into_any_element()
        };
        let zip_icon = if installing {
            spinner().with_size(px(14.0)).into_any_element()
        } else {
            Icon::new(icons::UPLOAD).size(px(14.0)).into_any_element()
        };

        div()
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .gap_3()
            .child(
                div()
                    .no_drag()
                    .relative()
                    .min_w_0()
                    .flex_1()
                    .child(Input::search(&self.search).cleanable(true).disabled(checking)),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .no_drag()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("refresh")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .has_icon(true)
                                    .disabled(checking)
                                    .h(px(32.0))
                                    .child(refresh_icon)
                                    .child("Refresh")
                                    .on_click(move |_, _, cx| {
                                        addons.update(cx, |s, cx| s.perform_bulk_update_check(cx))
                                    }),
                            )
                            .child(
                                Tooltip::new_with(|_, cx| {
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(Icon::new(icons::TRIANGLE_ALERT).size(px(16.0)))
                                        .child("Only install addons from sources you trust!")
                                        .text_color(cx.tokens().primary_foreground)
                                })
                                .id("install-zip-tip")
                                .side(crate::ui::components::Side::Bottom)
                                .child(
                                    Button::new("install-zip")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Sm)
                                        .has_icon(true)
                                        .disabled(installing)
                                        .h(px(32.0))
                                        .child(zip_icon)
                                        .child("Install ZIP")
                                        .on_click(|_, _, cx| install_zip::pick_and_install(cx)),
                                ),
                            ),
                    )
                    .children((count > 0).then(|| {
                        Badge::new(SharedString::from(format!(
                            "{count} {}",
                            if count == 1 { "addon" } else { "addons" }
                        )))
                        .variant(BadgeVariant::Secondary)
                        .icon(icons::PACKAGE)
                        .text_color(t.muted_foreground)
                        .px(px(8.0))
                        .py(px(4.0))
                    }))
                    .children((updates > 0).then(|| {
                        Badge::new(SharedString::from(format!(
                            "{updates} update{}",
                            if updates != 1 { "s" } else { "" }
                        )))
                        .variant(BadgeVariant::Outline)
                        .icon(icons::REFRESH_CW)
                        .text_color(t.primary)
                        .border_color(alpha(t.primary, 0.2))
                        .bg(alpha(t.primary, 0.1))
                        .px(px(8.0))
                        .py(px(4.0))
                    })),
            )
    }
}

/// Creates the toolbar view (put it into the title bar with `titlebar::set_slot`).
pub fn new_toolbar(
    model: Entity<DashboardModel>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> Entity<DashboardToolbar> {
    cx.new(|cx| DashboardToolbar::new(model, window, cx))
}
