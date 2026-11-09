//! The overlay that draws the toast queue (`ToastLayer`, a gpui-kit `Root` plugin).
use std::time::Duration;

use gpui_kit::base::RootPlugin;
use gpui_kit::component::IconNamed;
use gpui_kit::{
    Animation, AnimationExt, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Transformation, Window, deferred, div, ease_out_quint, percentage,
    prelude::FluentBuilder as _, px, svg,
};

use super::*;
use crate::{
    icons,
    theme::{Tokens, TokensExt, TwText, alpha, radius},
};

/// Renders every live toast at the bottom right, 24 px from the window edges, with 14 px between toasts.
pub(super) struct ToastLayer;

impl RootPlugin for ToastLayer {}

impl ToastLayer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        Self
    }
}

impl Render for ToastLayer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = store(cx).read(cx).items.clone();
        let t = cx.tokens();
        // Deferred: gpui-base paints dialogs as `deferred(..).with_priority(10 + layer)`, which draws after the whole
        // normal tree, so a plain plugin view would end up under them. 50 is above every dialog layer and below
        // popups (100) and tooltips (200), so selects/menus opened inside dialogs still win.
        deferred(
            div()
                .absolute()
                .right(px(24.0))
                .bottom(px(24.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .children(items.into_iter().map(move |item| toast_view(item, t))),
        )
        .with_priority(50)
    }
}

fn toast_view(item: ToastItem, t: Tokens) -> impl IntoElement {
    let id = item.id;
    let leaving = item.leaving;
    let toast = item.toast;
    let has_button = toast.button.is_some();
    let close_group = gpui_kit::SharedString::from(format!("toast-close-{id}"));

    let icon = match (toast.kind, toast.icon) {
        (ToastKind::Loading, _) => Some(
            svg()
                .path(icons::LOADER_CIRCLE.path())
                .size(px(20.0))
                .flex_none()
                .text_color(t.foreground)
                .with_animation(("toast-spin", id), Animation::new(Duration::from_millis(1000)).repeat(), |s, d| {
                    s.with_transformation(Transformation::rotate(percentage(d)))
                })
                .into_any_element(),
        ),
        (_, Some(i)) => {
            Some(svg().path(i.path()).size(px(20.0)).flex_none().text_color(t.foreground).into_any_element())
        }
        _ => None,
    };

    let el = div()
        .id(("toast", id))
        .relative()
        .flex()
        .items_center()
        .w(px(364.0))
        .overflow_hidden()
        .rounded(radius::LG)
        .border_1()
        .border_color(t.border)
        .bg(t.background)
        .p(px(16.0))
        .shadow_lg()
        .occlude()
        .text_color(t.foreground)
        .child(
            div()
                .id(("toast-close", id))
                .group(close_group.clone())
                .absolute()
                .right(px(8.0))
                .top(px(8.0))
                .p(px(4.0))
                .rounded(radius::MD)
                .cursor_pointer()
                .child(
                    svg()
                        .path(icons::X.path())
                        .size(px(16.0))
                        .text_color(alpha(t.foreground, 0.5))
                        .group_hover(close_group, move |s| s.text_color(t.foreground)),
                )
                .on_click(move |_, _, cx| dismiss(cx, id)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .min_w_0()
                .flex_1()
                .when_some(icon, |this, icon| this.child(icon))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .when(has_button, |d| d.mr(px(16.0)))
                        .child(
                            div()
                                .text_base_tw()
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .text_ellipsis()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(toast.title.clone()),
                        )
                        .child(div().text_sm_tw().text_color(t.muted_foreground).child(toast.description.clone())),
                ),
        )
        .when_some(toast.button.clone(), |this, b| {
            let on_click = b.on_click.clone();
            this.child(
                div()
                    .id(("toast-btn", id))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .h(px(32.0))
                    .px(px(12.0))
                    .rounded(radius::MD)
                    .bg(t.primary)
                    .text_color(t.primary_foreground)
                    .text_sm_tw()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(move |s| s.bg(alpha(t.primary, 0.9)))
                    .child(b.label)
                    .on_click(move |_, window, cx| {
                        on_click(window, cx);
                        dismiss(cx, id);
                    }),
            )
        });

    let anim_id = (if leaving { "toast-out" } else { "toast-in" }, id);
    el.with_animation(
        anim_id,
        Animation::new(Duration::from_millis(if leaving { 200 } else { 350 })).with_easing(ease_out_quint()),
        move |el, d| {
            if leaving { el.opacity(1.0 - d) } else { el.opacity(d).mb(px(-24.0 * (1.0 - d))) }
        },
    )
}
