//! `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`: a row of tab buttons plus the content of the active tab.
use std::rc::Rc;

use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::card::align_self_start;
use super::style_ext::StyleMergeExt as _;
use crate::theme::{TokensExt, TwText, alpha, radius, shadow_sm};

type ValueChange = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type StyleFn = Box<dyn Fn(StyleRefinement) -> StyleRefinement>;

/// Visual style of a tab list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsVariant {
    /// Rounded list on a muted background (36 px tall, 3 px padding). The active trigger gets a bordered, shadowed fill.
    #[default]
    Segmented,
    /// Rounded-full bordered pills (developer addon details). The active pill is tinted with the primary colour.
    Chip,
    /// Bare list with a 4 px gap and unstyled triggers (style them via `Styled` / `active_style`).
    Plain,
}

/// One tab button: `TabsTrigger::new("overview").icon(icons::LAYOUT_LIST).label("Overview")`.
/// `Styled` styles the trigger always, `.active_style(|s| s.bg(..))` only while it is the active one.
#[derive(IntoElement)]
pub struct TabsTrigger {
    value: SharedString,
    variant: TabsVariant,
    active: bool,
    icon: Option<Icon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
    on_select: Option<ValueChange>,
    active_style: Option<StyleFn>,
    style: StyleRefinement,
    list_id: Option<ElementId>,
}

impl TabsTrigger {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            variant: TabsVariant::Segmented,
            active: false,
            icon: None,
            label: None,
            children: Vec::new(),
            disabled: false,
            on_select: None,
            active_style: None,
            style: StyleRefinement::default(),
            list_id: None,
        }
    }

    /// Leading 16px icon.
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

    /// Extra styling applied only while the trigger is active, e.g. `|s| s.text_color(red)`.
    pub fn active_style(mut self, f: impl Fn(StyleRefinement) -> StyleRefinement + 'static) -> Self {
        self.active_style = Some(Box::new(f));
        self
    }
}

impl ParentElement for TabsTrigger {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TabsTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TabsTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (active, disabled) = (self.active, self.disabled);
        let id: ElementId = match self.list_id {
            Some(list) => (list, self.value.clone()).into(),
            None => ElementId::from(self.value.clone()),
        };
        let on_select = self.on_select;
        let value = self.value;

        let mut el = div()
            .id(id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .whitespace_nowrap()
            .text_sm_tw()
            .font_weight(FontWeight::MEDIUM)
            .when(disabled, |d| d.opacity(0.5))
            .when(!disabled, |d| d.cursor_pointer());

        match self.variant {
            TabsVariant::Segmented => {
                el = el
                    .h(px(29.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(radius::MD)
                    .border_1()
                    .border_color(gpui_kit::transparent_black())
                    .text_color(if active { t.foreground } else { t.muted_foreground });
                if active {
                    el = el.border_color(t.input).bg(alpha(t.input, 0.3)).shadow(shadow_sm());
                }
            }
            TabsVariant::Chip => {
                el = el.rounded_full().border_1().px(px(12.0)).py(px(6.0));
                if active {
                    el = el.border_color(alpha(t.primary, 0.4)).bg(alpha(t.primary, 0.15)).text_color(t.primary);
                } else {
                    let (fg, border) = (t.foreground, alpha(t.foreground, 0.2));
                    el = el.border_color(t.border).text_color(t.muted_foreground);
                    if !disabled {
                        el = el.hover(move |s| {
                            s.border_color(border).text_color(fg).text_sm_tw().font_weight(FontWeight::MEDIUM)
                        });
                    }
                }
            }
            TabsVariant::Plain => {}
        }

        el = el.merge_style(&self.style);
        if active && let Some(f) = &self.active_style {
            let mut s = StyleRefinement::default();
            s = f(s);
            el = el.merge_style(&s);
        }

        el.when(!disabled && !active, |d| {
            d.when_some(on_select, move |d, f| d.on_click(move |_, w, cx| f(&value, w, cx)))
        })
        .children(self.icon)
        .children(self.label)
        .children(self.children)
    }
}

/// Holds the triggers. Stand-alone use needs the caller to keep the active tab: `.value(active)` +
/// `.on_value_change(..)`. Inside [`Tabs`] these are wired automatically.
///
/// ```ignore
/// TabsList::new("dev-tabs").variant(TabsVariant::Chip).value(&self.tab)
///     .on_value_change(cx.listener(|this, v: &SharedString, _, cx| { this.tab = v.clone(); cx.notify(); }))
///     .trigger(TabsTrigger::new("overview").icon(icons::LAYOUT_LIST).label("Overview"))
///     .trigger(TabsTrigger::new("history").label("History"))
/// ```
#[derive(IntoElement)]
pub struct TabsList {
    id: ElementId,
    variant: TabsVariant,
    value: Option<SharedString>,
    on_value_change: Option<ValueChange>,
    triggers: Vec<TabsTrigger>,
    style: StyleRefinement,
}

impl TabsList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: TabsVariant::Segmented,
            value: None,
            on_value_change: None,
            triggers: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Currently active trigger value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn on_value_change(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }

    pub fn trigger(mut self, trigger: TabsTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }
}

impl Styled for TabsList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TabsList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let variant = self.variant;
        let list_id = self.id.clone();
        let base = div().id(self.id).flex().items_center();
        let base = match variant {
            TabsVariant::Segmented => base
                .h(px(36.0))
                .justify_center()
                .rounded(radius::LG)
                .bg(t.muted)
                .p(px(3.0))
                .text_color(t.muted_foreground),
            TabsVariant::Chip => base.gap(px(8.0)).flex_wrap(),
            TabsVariant::Plain => base.gap(px(4.0)),
        };
        let value = self.value;
        let on_change = self.on_value_change;
        base.merge_style(&self.style).children(self.triggers.into_iter().map(move |mut tr| {
            tr.variant = variant;
            tr.active = value.as_ref() == Some(&tr.value);
            tr.on_select = on_change.clone();
            tr.list_id = Some(list_id.clone());
            tr
        }))
    }
}

/// Content of one tab: takes the remaining space and is shown only while its tab is active (inside [`Tabs`]).
#[derive(IntoElement)]
pub struct TabsContent {
    value: SharedString,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl TabsContent {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self { value: value.into(), children: Vec::new(), style: StyleRefinement::default() }
    }
}

impl ParentElement for TabsContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TabsContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TabsContent {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_1().merge_style(&self.style).children(self.children)
    }
}

/// All-in-one tabs: a vertical stack (8 px gap) of the list and the active content. The widget keeps the active tab by
/// default (stored in the window under `id`, start with `.default_value(..)`). The caller can keep it instead with
/// `.value(..)` + `.on_value_change(..)`.
///
/// ```ignore
/// Tabs::new("t").default_value("account")
///     .list(TabsList::new("t-list").trigger(TabsTrigger::new("account").label("Account")).trigger(TabsTrigger::new("pw").label("Password")))
///     .content(TabsContent::new("account").child("Account form"))
///     .content(TabsContent::new("pw").child("Password form"))
/// ```
/// When the list and the panels live in different parts of the layout (header vs scroll area), use a stand-alone
/// [`TabsList`] and render the matching panel yourself (`match self.tab`).
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    value: Option<SharedString>,
    default_value: Option<SharedString>,
    on_value_change: Option<ValueChange>,
    variant: TabsVariant,
    list: Option<TabsList>,
    contents: Vec<TabsContent>,
    style: StyleRefinement,
}

impl Tabs {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            default_value: None,
            on_value_change: None,
            variant: TabsVariant::Segmented,
            list: None,
            contents: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn on_value_change(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }

    pub fn list(mut self, list: TabsList) -> Self {
        self.list = Some(list);
        self
    }

    pub fn content(mut self, content: TabsContent) -> Self {
        self.contents.push(content);
        self
    }
}

impl Styled for Tabs {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let initial = self
            .default_value
            .clone()
            .or_else(|| self.list.as_ref().and_then(|l| l.triggers.first().map(|t| t.value.clone())));
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| initial);
        let controlled = self.value.is_some();
        let active: Option<SharedString> = self.value.clone().or_else(|| state.read(cx).clone());
        let on_change = self.on_value_change;
        let variant = self.variant;

        let list = self.list.map(|list| {
            let state = state.clone();
            let on_change = on_change.clone();
            let mut list = list.variant(variant).on_value_change(move |v, window, cx| {
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
            list.value = active.clone();
            if variant == TabsVariant::Segmented {
                list = align_self_start(list);
            }
            list
        });

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(8.0))
            .merge_style(&self.style)
            .children(list)
            .children(self.contents.into_iter().filter(|c| Some(&c.value) == active.as_ref()))
    }
}
