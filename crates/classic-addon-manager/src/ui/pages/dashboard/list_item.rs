//! One row of the dashboard addon list: icon, name, version (or a "Manual" tag), author and an update status indicator.
use std::rc::Rc;

use cam_core::addon::Addon;
use cam_ui::{
    components::spinner,
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    App, ElementId, FontWeight, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::ui::addon_icon::{AddonIcon, AddonIconSize};

type ClickFn = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct AddonListItem {
    id: ElementId,
    addon: Addon,
    selected: bool,
    has_update: bool,
    checking_for_updates: bool,
    icon_url: Option<String>,
    on_click: ClickFn,
}

impl AddonListItem {
    pub fn new(
        id: impl Into<ElementId>,
        addon: Addon,
        selected: bool,
        has_update: bool,
        checking_for_updates: bool,
        icon_url: Option<String>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self { id: id.into(), addon, selected, has_update, checking_for_updates, icon_url, on_click: Rc::new(on_click) }
    }
}

impl RenderOnce for AddonListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (primary, border, muted_fg) = (t.primary, t.border, t.muted_foreground);
        let (muted, secondary, secondary_fg) = (t.muted, t.secondary, t.secondary_foreground);
        let addon = self.addon;
        let on_click = self.on_click;
        let selected = self.selected;

        let indicator = if addon.is_managed && self.checking_for_updates {
            spinner().color(primary).into_any_element()
        } else if addon.is_managed && self.has_update {
            Icon::new(icons::DOWNLOAD).size(px(16.0)).text_color(primary).into_any_element()
        } else if addon.is_managed {
            Icon::new(icons::CHECK).size(px(16.0)).text_color(tw(Tw::Green, 600)).into_any_element()
        } else {
            div().size(px(8.0)).rounded_full().bg(tw(Tw::Orange, 600)).into_any_element()
        };

        let version_row = if addon.is_managed {
            div()
                .min_w_0()
                .truncate()
                .text_xs_tw()
                .text_color(muted_fg)
                .child(SharedString::from(addon.version.clone()))
                .into_any_element()
        } else {
            div()
                .flex_none()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(radius::DEFAULT)
                .border_1()
                .border_color(secondary)
                .bg(alpha(secondary, 0.5))
                .text_color(secondary_fg)
                .text_size(px(9.0))
                .line_height(px(13.5))
                .font_weight(FontWeight::MEDIUM)
                .child("Manual")
                .into_any_element()
        };

        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(10.0))
            .w_full()
            .px(px(10.0))
            .py(px(8.0))
            .rounded(radius::LG)
            .border_1()
            .cursor_pointer()
            .when(selected, |d| d.bg(alpha(primary, 0.1)).border_color(alpha(primary, 0.3)))
            .when(!selected, |d| {
                d.border_color(gpui_kit::transparent_black())
                    .hover(move |s| s.bg(alpha(muted, 0.5)).border_color(alpha(border, 0.5)))
            })
            .on_click(move |_, window, cx| on_click(window, cx))
            .child(div().flex_none().child(
                AddonIcon::new(SharedString::from(addon.alias.clone()), self.icon_url.clone()).size(AddonIconSize::Sm),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm_tw()
                            .font_weight(FontWeight::MEDIUM)
                            .when(selected, |d| d.text_color(primary))
                            .child(SharedString::from(addon.alias.clone())),
                    )
                    .child(
                        div().flex().items_center().gap_2().mt(px(2.0)).child(version_row).child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_xs_tw()
                                .text_color(muted_fg)
                                .child(SharedString::from(addon.author.clone())),
                        ),
                    ),
            )
            .child(div().flex_none().flex().items_center().child(indicator))
    }
}
