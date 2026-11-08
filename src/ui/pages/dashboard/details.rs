//! Details pane of the selected addon: header with the action menu, a grid of facts (name, update time, source,
//! rating) and the addon's README. One view exists per `name:repo:branch` key (see [`DetailsPane::key_of`]), and the
//! page recreates it when the key changes.
use gpui_kit::component::Icon;
use gpui_kit::{
    AppContext as _, Context, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::{atoms::DashboardModel, repo_match::RepoMatch, update_dialog, version_dialog};
use crate::{
    backend::{
        addon::{self, Addon},
        api::Release,
    },
    state::{self, AddonState, Load, queries, ratings::RatingModel, run_blocking, tasks::error_message},
    ui::{
        components::{
            AddonIcon, AddonIconSize, Align, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, DropdownMenu,
            MenuItem, RatingButtons, RatingSize, Readme, ScrollArea, Side,
        },
        icons,
        theme::{TokensExt, Tw, TwText, alpha, radius, shadow_sm, tw},
        toast,
        utils::format_to_local_time,
    },
};

pub struct DetailsPane {
    /// The addon the pane was created for; the live copy from the store wins while it is installed.
    snapshot: Addon,
    addons: Entity<AddonState>,
    readme: Entity<queries::ReadmeQuery>,
    rating: Entity<RatingModel>,
    repo_match: Option<Entity<RepoMatch>>,
    menu_open: bool,
}

impl DetailsPane {
    pub fn new(addon: Addon, model: Entity<DashboardModel>, cx: &mut Context<Self>) -> Self {
        let addons = state::addons(cx);
        cx.observe(&addons, |_, _, cx| cx.notify()).detach();

        let readme = queries::readme(cx);
        cx.observe(&readme, |_, _, cx| cx.notify()).detach();
        cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()).detach();
        if !addon.repo.is_empty() {
            let key = (addon.repo.clone(), addon.branch.clone());
            readme.update(cx, |q, cx| q.ensure(key, cx));
        }

        let rating = {
            let (name, alias, managed) = (addon.name.clone(), addon.alias.clone(), addon.is_managed);
            cx.new(|cx| RatingModel::new(name, alias, managed, cx))
        };
        cx.observe(&rating, |_, _, cx| cx.notify()).detach();

        let repo_match = (!addon.is_managed).then(|| {
            let name = addon.name.clone();
            cx.new(|cx| RepoMatch::new(name, model, cx))
        });

        Self { snapshot: addon, addons, readme, rating, repo_match, menu_open: false }
    }

    /// Key identifying the view: `name:repo:branch`.
    pub fn key_of(addon: &Addon) -> String {
        format!("{}:{}:{}", addon.name, addon.repo, addon.branch)
    }

    fn current(&self, cx: &gpui_kit::App) -> Addon {
        self.addons
            .read(cx)
            .installed_addons
            .iter()
            .find(|a| a.name == self.snapshot.name)
            .cloned()
            .unwrap_or_else(|| self.snapshot.clone())
    }

    fn open_directory(addon: &Addon, cx: &mut gpui_kit::App) {
        let (name, alias) = (addon.name.clone(), addon.alias.clone());
        let backend = state::backend(cx);
        cx.spawn(async move |cx| {
            let target = name.clone();
            if let Err(e) = run_blocking(cx, move || backend.addon_dir()?.open_directory(&target)).await {
                log::error!("Opening the directory of {name} failed: {e:#}");
                cx.update(|cx| {
                    toast::show(
                        cx,
                        toast::Toast::new("Error", format!("Failed to open directory for addon \"{alias}\"."))
                            .icon(icons::TRIANGLE_ALERT),
                    )
                });
            }
        })
        .detach();
    }

    fn uninstall(addon: Addon, cx: &mut gpui_kit::App) {
        let (name, alias) = (addon.name.clone(), addon.alias.clone());
        let uninstall = state::addons(cx).update(cx, |s, cx| s.uninstall(name.clone(), cx));
        cx.spawn(async move |cx| {
            let res = uninstall.await;
            cx.update(|cx| match res {
                Ok(()) => {
                    toast::show(
                        cx,
                        toast::Toast::new("Addon uninstalled", format!("{alias} was uninstalled")).icon(icons::CHECK),
                    );
                }
                Err(e) => {
                    log::error!("Uninstalling {name} failed: {e:#}");
                    toast::show(
                        cx,
                        toast::Toast::new("Error", format!("Failed to uninstall addon: {}", error_message(&e)))
                            .icon(icons::TRIANGLE_ALERT),
                    );
                }
            })
        })
        .detach();
    }

    fn unmanage(addon: Addon, cx: &mut gpui_kit::App) {
        let (name, alias) = (addon.name.clone(), addon.alias.clone());
        let unmanage = state::addons(cx).update(cx, |s, cx| s.unmanage(name.clone(), cx));
        cx.spawn(async move |cx| {
            let res = unmanage.await;
            cx.update(|cx| match res {
                Ok(()) => {
                    toast::show(
                        cx,
                        toast::Toast::new("Addon unmanaged", format!("{alias} is no longer managed"))
                            .icon(icons::CHECK),
                    );
                }
                Err(e) => {
                    log::error!("Unmanaging {name} failed: {e:#}");
                    toast::show(
                        cx,
                        toast::Toast::new("Error", format!("Failed to unmanage addon: {}", error_message(&e)))
                            .icon(icons::TRIANGLE_ALERT),
                    );
                }
            })
        })
        .detach();
    }
}

/// One cell of the facts grid: a muted label above a bold value that is cut off with an ellipsis when too long.
fn cell(label: impl IntoElement, value: impl IntoElement) -> gpui_kit::Div {
    div().w(px(244.0)).child(label).child(value)
}

impl Render for DetailsPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let addon = self.current(cx);
        let store = self.addons.read(cx);
        let latest_release: Option<Release> = store.latest_releases.get(&addon.name).cloned();
        let checking = store.is_checking_for_updates;
        let has_update = latest_release.as_ref().is_some_and(|r| addon::has_update(&addon, r));
        let icon_url = state::catalog::current(cx).and_then(|c| c.icon_url(&addon.name).map(str::to_owned));
        let this = cx.entity();

        let green = tw(Tw::Green, 600);
        let orange = tw(Tw::Orange, 600);

        // ---------------------------------------------------------------- header
        let mut meta =
            div().flex().items_center().gap_x_3().mt_1().overflow_hidden().text_sm_tw().text_color(t.muted_foreground);
        if addon.is_managed {
            meta =
                meta.child(
                    div().flex().items_center().gap_1().child(Icon::new(icons::USER).size(px(14.0))).child(
                        div().text_color(alpha(t.foreground, 0.9)).child(SharedString::from(addon.author.clone())),
                    ),
                );
            if !addon.version.is_empty() {
                meta = meta.child(
                    Badge::new(SharedString::from(addon.version.clone()))
                        .variant(BadgeVariant::Secondary)
                        .icon(icons::TAG),
                );
            }
        } else {
            meta = meta.child(
                Badge::new("Manual")
                    .variant(BadgeVariant::Outline)
                    .text_color(orange)
                    .border_color(alpha(orange, 0.2))
                    .bg(alpha(tw(Tw::Orange, 500), 0.1)),
            );
        }
        if addon.is_managed && !has_update && latest_release.is_some() && !checking {
            meta = meta.child(
                Badge::new("Up to date")
                    .variant(BadgeVariant::Outline)
                    .text_color(green)
                    .border_color(alpha(green, 0.2))
                    .bg(alpha(tw(Tw::Green, 500), 0.1)),
            );
        }

        let update_badge = latest_release.clone().filter(|_| has_update).map(|release| {
            let tag = release.tag_name.clone();
            let addon = addon.clone();
            let primary = t.primary;
            div().flex().mt_2().child(
                div()
                    .id("update-badge")
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px_3()
                    .py_1()
                    .rounded(radius::MD)
                    .border_1()
                    .border_color(alpha(primary, 0.2))
                    .bg(alpha(primary, 0.1))
                    .text_sm_tw()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(primary)
                    .cursor_pointer()
                    .hover(move |s| s.bg(alpha(primary, 0.2)))
                    .on_click(move |_, window, cx| update_dialog::open(addon.clone(), release.clone(), window, cx))
                    .child(Icon::new(icons::DOWNLOAD).size(px(16.0)))
                    .child(SharedString::from(format!("Update to {tag}"))),
            )
        });

        let menu = {
            let (a_dir, a_issue, a_version, a_unmanage, a_uninstall) =
                (addon.clone(), addon.clone(), addon.clone(), addon.clone(), addon.clone());
            let managed = addon.is_managed;
            let open_this = this.clone();
            DropdownMenu::new()
                .id("addon-actions")
                .side(Side::Bottom)
                .align(Align::End)
                .open(self.menu_open)
                .on_open_change(move |open, _, cx| {
                    open_this.update(cx, |t, cx| {
                        t.menu_open = open;
                        cx.notify();
                    })
                })
                .trigger(
                    Button::new("addon-menu")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Icon)
                        .flex_none()
                        .when(self.menu_open, |b| b.bg(t.accent))
                        .child(Icon::new(icons::MENU).size(px(20.0))),
                )
                .menu(move |m| {
                    let (a_dir, a_issue, a_version, a_unmanage, a_uninstall) =
                        (a_dir.clone(), a_issue.clone(), a_version.clone(), a_unmanage.clone(), a_uninstall.clone());
                    let mut m = m.item(
                        MenuItem::new("Open Directory")
                            .icon(icons::FOLDER_OPEN)
                            .on_click(move |_, cx| Self::open_directory(&a_dir, cx)),
                    );
                    if managed {
                        m = m
                            .item(MenuItem::new("Report Issue").icon(icons::BUG).on_click(move |_, cx| {
                                cx.open_url(&format!("https://github.com/{}/issues/new", a_issue.repo));
                            }))
                            .item(
                                MenuItem::new("Install Version")
                                    .icon(icons::GIT_BRANCH)
                                    .on_click(move |window, cx| version_dialog::open(a_version.clone(), window, cx)),
                            )
                            .item(
                                MenuItem::new("Unmanage")
                                    .warning()
                                    .icon(icons::TRIANGLE_ALERT)
                                    .on_click(move |_, cx| Self::unmanage(a_unmanage.clone(), cx)),
                            );
                    }
                    m.separator().item(
                        MenuItem::new("Uninstall")
                            .destructive()
                            .icon(icons::TRASH_2)
                            .on_click(move |_, cx| Self::uninstall(a_uninstall.clone(), cx)),
                    )
                })
        };

        let header = div().flex_none().border_b_1().border_color(t.border).px_6().py_5().child(
            div()
                .flex()
                .items_start()
                .gap_4()
                .child(
                    div().flex_none().child(
                        AddonIcon::new(SharedString::from(addon.alias.clone()), icon_url.clone())
                            .size(AddonIconSize::Lg)
                            .shadow(shadow_sm()),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_xl_tw()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(addon.alias.clone())),
                        )
                        .child(meta)
                        .children(update_badge),
                )
                .child(div().flex_none().child(menu)),
        );

        // ---------------------------------------------------------------- facts grid
        let label = |text: &'static str| div().text_color(t.muted_foreground).child(text);
        let value = |text: String| div().truncate().font_weight(FontWeight::MEDIUM).child(SharedString::from(text));
        let mut grid = div()
            .flex()
            .flex_wrap()
            .gap_x_6()
            .gap_y_2()
            .w(px(512.0))
            .text_sm_tw()
            .child(cell(label("Name"), value(addon.name.clone())));
        if addon.is_managed {
            let updated = addon.updated_at.map_or_else(|| "—".to_string(), |t| format_to_local_time(&t, true));
            grid = grid.child(cell(label("Updated"), value(updated)));
            let source_label = div()
                .flex()
                .items_center()
                .gap_1()
                .text_color(t.muted_foreground)
                .child(Icon::new(icons::GITHUB).size(px(14.0)))
                .child("Source");
            let source_value = if addon.repo.is_empty() {
                value("—".into()).into_any_element()
            } else {
                let url = format!("https://github.com/{}", addon.repo);
                let blue = tw(Tw::Blue, 400);
                let blue_hover = tw(Tw::Blue, 500);
                div()
                    .id("addon-source-link")
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(blue)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(blue_hover).underline().text_sm_tw().font_weight(FontWeight::MEDIUM))
                    .on_click(move |_, _, cx| cx.open_url(&url))
                    .child(SharedString::from(addon.repo.clone()))
                    .into_any_element()
            };
            grid = grid.child(cell(source_label, source_value));
            let rating_model = self.rating.read(cx);
            let (rating, busy) = (rating_model.rating(cx), rating_model.busy(cx));
            let rating_entity = self.rating.clone();
            grid = grid.child(cell(
                label("Rating"),
                div().font_weight(FontWeight::MEDIUM).child(
                    RatingButtons::new("addon-rating")
                        .rating(rating as i32)
                        .disabled(busy)
                        .size(RatingSize::Sm)
                        .on_rate(move |value, _, cx| rating_entity.update(cx, |m, cx| m.rate(value, cx))),
                ),
            ));
        } else {
            grid = grid.child(cell(label("State"), div().font_weight(FontWeight::MEDIUM).child("Not managed")));
        }
        let facts = div().flex_none().border_b_1().border_color(t.border).px_6().py_3().child(grid);

        // ---------------------------------------------------------------- body
        let fallback = if addon.description.is_empty() {
            "No description provided".to_string()
        } else {
            addon.description.clone()
        };
        let body = if addon.is_managed {
            let readme = self.readme.read(cx).get(&(addon.repo.clone(), addon.branch.clone()));
            let readme_el = if !addon.repo.is_empty() && matches!(readme, Load::Idle | Load::Pending) {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .h(px(128.0))
                    .text_center()
                    .text_color(t.muted_foreground)
                    .child(Icon::new(icons::PACKAGE).size(px(40.0)).opacity(0.5).mb_3())
                    .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child("Loading description..."))
                    .into_any_element()
            } else {
                let text = if addon.repo.is_empty() {
                    fallback
                } else {
                    match readme {
                        Load::Ready(Some(text)) => text.clone(),
                        _ => fallback,
                    }
                };
                div()
                    .child(
                        Readme::new(SharedString::from(format!("readme-{}", addon.name)), SharedString::from(text))
                            .repo(addon.repo.clone())
                            .branch(addon.branch.clone()),
                    )
                    .into_any_element()
            };
            ScrollArea::new("details-scroll")
                .flex_1()
                .min_h_0()
                .content_style(|d| d.px_6().py_4())
                .child(readme_el)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .px_6()
                .py_4()
                .children(self.repo_match.clone())
                .into_any_element()
        };

        div().flex().flex_col().size_full().overflow_hidden().child(header).child(facts).child(body)
    }
}
