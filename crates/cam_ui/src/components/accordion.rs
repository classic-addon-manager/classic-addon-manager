//! `Accordion`: vertically stacked collapsible items with a rotating chevron.
use std::rc::Rc;

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Transformation, Window, div, percentage,
    prelude::FluentBuilder as _, px,
};

use super::style_ext::StyleMergeExt as _;
use crate::{
    icons,
    theme::{TokensExt, TwText, radius},
};

type ValueChange = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// One item of an [`Accordion`]: `AccordionItem::new("a").trigger("Title").content(div().child("Body"))`.
/// `trigger` is any element (text or a custom row), the chevron is added by the accordion.
pub struct AccordionItem {
    value: SharedString,
    trigger: Option<AnyElement>,
    content: Option<Box<dyn FnOnce() -> AnyElement>>,
    disabled: bool,
}

impl AccordionItem {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self { value: value.into(), trigger: None, content: None, disabled: false }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        let el = content.into_any_element();
        self.content = Some(Box::new(move || el));
        self
    }

    /// Content built lazily, only while the item is open.
    pub fn content_with(mut self, f: impl FnOnce() -> AnyElement + 'static) -> Self {
        self.content = Some(Box::new(f));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A list of items of which one is open at a time by default. The open item cannot be closed unless
/// `.collapsible(true)` is set, and `.multiple(true)` allows several open items.
///
/// State: either kept by the widget (stored in the window keyed by `id`, initial `.default_value(["a"])`) or by the
/// caller with `.value(vec!["a".into()])` + `.on_value_change(|values, window, cx| ..)`.
///
/// ```ignore
/// Accordion::new("faq").collapsible(true).default_value(["q1"])
///     .item(AccordionItem::new("q1").trigger("Question?").content("Answer"))
///     .item(AccordionItem::new("q2").trigger("Another").content("Answer 2"))
/// ```
/// Content shows and hides instantly (no height animation).
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    multiple: bool,
    collapsible: bool,
    value: Option<Vec<SharedString>>,
    default_value: Vec<SharedString>,
    on_value_change: Option<ValueChange>,
    items: Vec<AccordionItem>,
    style: StyleRefinement,
}

impl Accordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            multiple: false,
            collapsible: false,
            value: None,
            default_value: Vec::new(),
            on_value_change: None,
            items: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Allows closing the open item (when `.multiple(true)` is not set).
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    pub fn default_value<S: Into<SharedString>>(mut self, values: impl IntoIterator<Item = S>) -> Self {
        self.default_value = values.into_iter().map(Into::into).collect();
        self
    }

    /// Open items, kept by the caller.
    pub fn value(mut self, values: Vec<SharedString>) -> Self {
        self.value = Some(values);
        self
    }

    pub fn on_value_change(mut self, f: impl Fn(&[SharedString], &mut Window, &mut App) + 'static) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }

    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }
}

impl Styled for Accordion {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let default_value = self.default_value.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| default_value);
        let controlled = self.value.is_some();
        let current: Vec<SharedString> = self.value.clone().unwrap_or_else(|| state.read(cx).clone());
        let (multiple, collapsible) = (self.multiple, self.collapsible);
        let root_id = self.id.clone();

        let rows = self.items.into_iter().map(|item| {
            let open = current.contains(&item.value);
            let value = item.value.clone();
            let disabled = item.disabled;
            let current = current.clone();
            let state = state.clone();
            let on_change = self.on_value_change.clone();

            let trigger = div()
                .id((root_id.clone(), item.value.clone()))
                .flex()
                .flex_1()
                .items_start()
                .justify_between()
                .gap(px(16.0))
                .rounded(radius::MD)
                .py(px(16.0))
                .text_sm_tw()
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.foreground)
                .when(disabled, |d| d.opacity(0.5))
                .when(!disabled, |d| {
                    d.cursor_pointer()
                        .hover(move |s| {
                            s.underline().text_sm_tw().font_weight(FontWeight::MEDIUM).text_color(t.foreground)
                        })
                        .on_click(move |_, window, cx| {
                            let mut next = current.clone();
                            if multiple {
                                if open {
                                    next.retain(|v| *v != value);
                                } else {
                                    next.push(value.clone());
                                }
                            } else if open {
                                if collapsible {
                                    next.clear();
                                }
                            } else {
                                next = vec![value.clone()];
                            }
                            if !controlled {
                                state.update(cx, |s, cx| {
                                    *s = next.clone();
                                    cx.notify();
                                });
                            }
                            if let Some(cb) = &on_change {
                                cb(&next, window, cx);
                            }
                        })
                })
                .child(div().flex_1().children(item.trigger))
                .child(
                    div().mt(px(2.0)).flex_none().child(
                        Icon::new(icons::CHEVRON_DOWN)
                            .size(px(16.0))
                            .text_color(t.muted_foreground)
                            .when(open, |i| i.transform(Transformation::rotate(percentage(0.5)))),
                    ),
                );

            div().border_b_1().border_color(t.border).child(div().flex().child(trigger)).when(open, |d| {
                d.children(
                    item.content.map(|f| div().overflow_hidden().text_sm_tw().child(div().pb(px(16.0)).child(f()))),
                )
            })
        });

        div().id(self.id).merge_style(&self.style).children(rows)
    }
}
