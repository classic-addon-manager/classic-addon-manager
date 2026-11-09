//! `ToggleGroup` / `ToggleGroupItem`: a row of joined buttons of which exactly one is active.
use std::rc::Rc;

use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::style_ext::StyleMergeExt as _;
use super::toggle::{ToggleSize, ToggleVariant, toggle_root};
use crate::theme::{TokensExt, radius};

type ValueChange = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One button of a [`ToggleGroup`]: `ToggleGroupItem::new("grid").icon(icons::LAYOUT_GRID)` / `.label("All")`. Implements
/// `Styled`, so callers can chain styling methods.
#[derive(IntoElement)]
pub struct ToggleGroupItem {
    value: SharedString,
    icon: Option<Icon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
    style: StyleRefinement,
    // filled by the group
    group_id: Option<ElementId>,
    variant: ToggleVariant,
    size: ToggleSize,
    on: bool,
    first: bool,
    last: bool,
    group_disabled: bool,
    on_select: Option<ValueChange>,
}

impl ToggleGroupItem {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            icon: None,
            label: None,
            children: Vec::new(),
            disabled: false,
            style: StyleRefinement::default(),
            group_id: None,
            variant: ToggleVariant::Default,
            size: ToggleSize::Default,
            on: false,
            first: true,
            last: true,
            group_disabled: false,
            on_select: None,
        }
    }

    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(Icon::new(icon).size(px(16.0)));
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for ToggleGroupItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ToggleGroupItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToggleGroupItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let disabled = self.disabled || self.group_disabled;
        let id = match self.group_id {
            Some(g) => (g, self.value.clone()).into(),
            None => ElementId::from(self.value.clone()),
        };
        let outline = self.variant == ToggleVariant::Outline;
        let (first, last) = (self.first, self.last);
        let value = self.value;
        let on_select = self.on_select;
        let on = self.on;
        let mut el = toggle_root(id, self.variant, self.size, on, disabled, &t)
            // Items sit directly next to each other: square inner corners, no shadow, shared borders
            .min_w_0()
            .px(px(12.0))
            .rounded(px(0.0))
            .shadow(vec![]);
        if first {
            el = el.rounded_tl(radius::MD).rounded_bl(radius::MD);
        }
        if last {
            el = el.rounded_tr(radius::MD).rounded_br(radius::MD);
        }
        if outline && !first {
            el = el.border_l_0();
        }
        el.when(!disabled && !on, |d| d.when_some(on_select, move |d, f| d.on_click(move |_, w, cx| f(&value, w, cx))))
            .merge_style(&self.style)
            .children(self.icon)
            .children(self.label)
            .children(self.children)
    }
}

/// A row of joined buttons of which exactly one is active (the list/grid switch of the addons toolbar).
///
/// State: either kept by the widget (`.default_value("list")`, stored in the window by `id`) or by the caller
/// (`.value("list")` + `.on_value_change(|value, window, cx| ..)`). The callback fires in both cases. Clicking the
/// active item does nothing. Several active items at once are not supported (use several `Toggle`s).
///
/// ```ignore
/// ToggleGroup::new("view-mode").variant(ToggleVariant::Outline).size(ToggleSize::Sm).value(mode)
///     .on_value_change(cx.listener(|this, v: &SharedString, _, cx| this.set_mode(v, cx)))
///     .item(ToggleGroupItem::new("list").icon(icons::LIST))
///     .item(ToggleGroupItem::new("grid").icon(icons::LAYOUT_GRID))
/// ```
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    variant: ToggleVariant,
    size: ToggleSize,
    value: Option<SharedString>,
    default_value: Option<SharedString>,
    disabled: bool,
    on_value_change: Option<ValueChange>,
    items: Vec<ToggleGroupItem>,
    style: StyleRefinement,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ToggleVariant::Default,
            size: ToggleSize::Default,
            value: None,
            default_value: None,
            disabled: false,
            on_value_change: None,
            items: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_value_change(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }

    pub fn item(mut self, item: ToggleGroupItem) -> Self {
        self.items.push(item);
        self
    }
}

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_value = self.default_value.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| default_value);
        let controlled = self.value.is_some();
        let current: Option<SharedString> = self.value.clone().or_else(|| state.read(cx).clone());
        let on_change = self.on_value_change;
        let (variant, size, disabled) = (self.variant, self.size, self.disabled);
        let group_id = self.id.clone();
        let n = self.items.len();

        let select: ValueChange = Rc::new(move |v, window, cx| {
            if !controlled {
                state.update(cx, |s, cx| {
                    *s = Some(v.clone());
                    cx.notify();
                });
            }
            if let Some(cb) = &on_change {
                cb(v, window, cx);
            }
        });

        div().id(self.id).flex().items_center().merge_style(&self.style).children(
            self.items.into_iter().enumerate().map(move |(i, mut item)| {
                item.group_id = Some(group_id.clone());
                item.variant = variant;
                item.size = size;
                item.on = current.as_ref() == Some(&item.value);
                item.first = i == 0;
                item.last = i + 1 == n;
                item.group_disabled = disabled;
                item.on_select = Some(select.clone());
                item
            }),
        )
    }
}
