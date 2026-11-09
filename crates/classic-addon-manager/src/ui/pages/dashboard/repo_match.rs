//! Offers to take over updates of a manually installed addon that exists in the catalog.
use cam_core::{addon::InstallError, api::AddonManifest};
use cam_ui::{
    components::Button,
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
    toast,
};
use gpui_kit::component::Icon;
use gpui_kit::{
    Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};

use super::model::DashboardModel;
use crate::{
    state::{self, catalog, tasks::error_message},
    ui::{dependency_notify::notify_dependency_result, format::truncate_chars},
};

pub struct RepoMatch {
    name: String,
    /// Guards against double submits: a second install while the first runs is useless.
    matching: bool,
    model: Entity<DashboardModel>,
}

impl RepoMatch {
    pub fn new(name: String, model: Entity<DashboardModel>, cx: &mut Context<Self>) -> Self {
        catalog::ensure(cx);
        cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()).detach();
        Self { name, matching: false, model }
    }

    /// Looks the addon up in the catalog and installs its latest release, which makes it a managed addon.
    fn handle_match(&mut self, cx: &mut Context<Self>) {
        if self.matching {
            return;
        }
        self.matching = true;
        let name = self.name.clone();
        let lookup = catalog::fetch(cx);
        cx.spawn(async move |this, cx| {
            let manifest = lookup.await.ok().and_then(|c| c.manifest(&name).cloned());
            let _ = this.update(cx, |this, cx| {
                let Some(manifest) = manifest else {
                    this.matching = false;
                    toast::show(
                        cx,
                        toast::Toast::new("Error", format!("Failed to fetch addon manifest for \"{name}\"."))
                            .icon(icons::TRIANGLE_ALERT),
                    );
                    return;
                };
                this.install(manifest, cx);
            });
        })
        .detach();
    }

    fn install(&mut self, manifest: AddonManifest, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let (name, alias) = (manifest.name.clone(), manifest.alias.clone());
        let install = state::addons(cx).update(cx, |s, cx| s.install(manifest, "latest".into(), cx));
        cx.spawn(async move |this, cx| {
            let res = install.await;
            let _ = this.update(cx, |t, _| t.matching = false);
            cx.update(|cx| match res {
                Ok(result) => {
                    if !notify_dependency_result(&result, &format!("Failed to match addon \"{name}\""), cx) {
                        return;
                    }
                    let description = format!("\"{alias}\" is now managed by Classic Addon Manager.");
                    toast::show(cx, toast::Toast::new("Addon Matched", description).icon(icons::CHECK));
                    model.update(cx, |m, cx| m.set_selected(None, cx));
                }
                Err(err) => {
                    state::log_failure(&format!("Matching {name} failed:"), &err);
                    let description = if InstallError::from_error(&err) == InstallError::NoRelease {
                        format!("No releases found for addon \"{name}\".")
                    } else {
                        format!("Failed to match addon \"{name}\": {}", truncate_chars(&error_message(&err), 100))
                    };
                    toast::show(cx, toast::Toast::new("Error", description).icon(icons::TRIANGLE_ALERT));
                }
            })
        })
        .detach();
    }
}

impl Render for RepoMatch {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        // Only an addon the catalog knows can be matched. Nothing shows while the catalog loads or is unreachable.
        if state::catalog::current(cx).is_none_or(|c| c.manifest(&self.name).is_none()) {
            return div().into_any_element();
        }
        let orange = tw(Tw::Orange, 500);
        let this = cx.entity();
        div()
            .flex_1()
            .flex()
            .flex_col()
            .p_4()
            .rounded(radius::LG)
            .border_1()
            .border_color(t.border)
            .bg(t.card)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .text_center()
                    .py_8()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(48.0))
                            .rounded_full()
                            .bg(alpha(orange, 0.1))
                            .mb_4()
                            .child(Icon::new(icons::CLOUD_OFF).size(px(24.0)).text_color(orange)),
                    )
                    .child(
                        div()
                            .text_sm_tw()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.foreground)
                            .mb_1()
                            .child("Manage This Addon?"),
                    )
                    .child(
                        div()
                            .text_sm_tw()
                            .text_color(t.muted_foreground)
                            .mb(px(20.0))
                            .max_w(px(320.0))
                            .child("You installed this one yourself.")
                            .child("We can take over updates if you'd like."),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm_tw()
                                    .text_color(t.muted_foreground)
                                    .child("It's in our repository, we can keep it up to date for you."),
                            )
                            .child(
                                Button::new("match-addon")
                                    .icon(icons::CIRCLE_ARROW_UP)
                                    .label(SharedString::from("Yes, keep it updated"))
                                    .on_click(move |_, _, cx| this.update(cx, |t, cx| t.handle_match(cx))),
                            ),
                    ),
            )
            .into_any_element()
    }
}
