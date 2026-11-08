//! Tooltip: a small rounded bubble in the primary colours (12 px by 6 px padding, extra small text) with a 10 px arrow.
//! It opens immediately on hover and closes when the pointer leaves or the trigger is pressed.
//!
//! ```ignore
//! Tooltip::new("Delete addon").child(Button::new("del").icon(icons::TRASH_2))            // text
//! Tooltip::new_with(|window, cx| div().child("rich").child("content")).side(Side::Bottom).child(el)  // builder
//! ```
//! `Tooltip` is a wrapper element around the trigger (style it with `.w_full()`, `.flex_1()`... like the trigger
//! would be). Options: `.side(Side::Right)`, `.align(..)`, `.side_offset(6.0)`, `.max_width(px(320.))` (enables
//! wrapping at that width), `.arrow(false)`, `.disabled(true)`. Give `.id(..)` when created in a loop (default id is the
//! constructor call site).
use std::{panic::Location, rc::Rc, time::Duration};

use super::style_ext::StyleMergeExt as _;
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, App, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement, PathBuilder, Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, canvas, div, point, prelude::FluentBuilder as _, px,
};

use super::floating::{Align, FloatState, Floating, Placement, Side, TOOLTIP_PRIORITY, element_id, track_bounds};
use crate::ui::theme::{TokensExt, TwText, radius};

type ContentFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

#[derive(IntoElement)]
pub struct Tooltip {
    id: ElementId,
    content: ContentFn,
    side: Side,
    align: Align,
    side_offset: f32,
    max_width: Option<Pixels>,
    arrow: bool,
    disabled: bool,
    default_open: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Tooltip {
    /// Tooltip with a text label.
    #[track_caller]
    pub fn new(text: impl Into<SharedString>) -> Self {
        let text: SharedString = text.into();
        Self::with_content(Location::caller(), Rc::new(move |_, _| div().child(text.clone()).into_any_element()))
    }

    /// Tooltip with custom content (a builder called while the tooltip is open).
    #[track_caller]
    pub fn new_with<E: IntoElement>(content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self {
        Self::with_content(Location::caller(), Rc::new(move |w, cx| content(w, cx).into_any_element()))
    }

    fn with_content(loc: &'static Location<'static>, content: ContentFn) -> Self {
        Self {
            id: element_id(None, loc),
            content,
            side: Side::Top,
            align: Align::Center,
            side_offset: 0.0,
            max_width: None,
            arrow: true,
            disabled: false,
            default_open: false,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Which side of the trigger the tooltip opens on (default `Top`). It flips to the opposite side when it does not
    /// fit.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Gap between the trigger and the tooltip in px (default 0, because the arrow overlaps the trigger by about 5 px).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    /// Wrap the text at this width (for example `px(320.)`). By default the tooltip is one line.
    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    /// Starts open until the pointer enters/leaves the trigger (visual tests).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Never show (e.g. when the trigger is already explained elsewhere).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Tooltip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The arrow: a triangle 10px along the tooltip edge and 5px deep, pointing away from the tooltip body.
fn arrow(side: Side, color: gpui_kit::Hsla) -> AnyElement {
    let (w, h) = match side {
        Side::Top | Side::Bottom => (px(10.0), px(5.0)),
        Side::Left | Side::Right => (px(5.0), px(10.0)),
    };
    let tri = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (o, s) = (bounds.origin, bounds.size);
            let mut b = PathBuilder::fill();
            let pts = match side {
                // tooltip above the trigger: arrow at its bottom edge pointing down
                Side::Top => [point(o.x, o.y), point(o.x + s.width, o.y), point(o.x + s.width / 2.0, o.y + s.height)],
                Side::Bottom => {
                    [point(o.x, o.y + s.height), point(o.x + s.width, o.y + s.height), point(o.x + s.width / 2.0, o.y)]
                }
                Side::Left => [point(o.x, o.y), point(o.x, o.y + s.height), point(o.x + s.width, o.y + s.height / 2.0)],
                Side::Right => {
                    [point(o.x + s.width, o.y), point(o.x + s.width, o.y + s.height), point(o.x, o.y + s.height / 2.0)]
                }
            };
            b.move_to(pts[0]);
            b.line_to(pts[1]);
            b.line_to(pts[2]);
            b.close();
            if let Ok(path) = b.build() {
                window.paint_path(path, color);
            }
        },
    )
    .w(w)
    .h(h)
    .flex_none();
    let holder = div().absolute().flex();
    match side {
        Side::Top => holder.left_0().right_0().bottom(px(-5.0)).justify_center(),
        Side::Bottom => holder.left_0().right_0().top(px(-5.0)).justify_center(),
        Side::Left => holder.top_0().bottom_0().right(px(-5.0)).items_center(),
        Side::Right => holder.top_0().bottom_0().left(px(-5.0)).items_center(),
    }
    .child(tri)
    .into_any_element()
}

impl RenderOnce for Tooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let default_open = self.default_open;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| FloatState::new(default_open, cx));
        let (open, anchor, measured) = {
            let s = state.read(cx);
            (s.open && !self.disabled, s.anchor, s.measured.clone())
        };

        let set = |state: &gpui_kit::Entity<FloatState>, open: bool, cx: &mut App| {
            state.update(cx, |s, cx| {
                if s.open != open {
                    s.open = open;
                    cx.notify();
                }
            });
        };

        let body = open.then(|| {
            let content = (self.content)(window, cx);
            let side = self.side;
            let mut bubble = div()
                .relative()
                .bg(t.primary)
                .text_color(t.primary_foreground)
                .rounded(radius::MD)
                .px(px(12.0))
                .py(px(6.0))
                .text_xs_tw()
                .when(self.max_width.is_none(), |d| d.whitespace_nowrap())
                .when_some(self.max_width, |d, w| d.max_w(w))
                .child(content);
            if self.arrow {
                bubble = bubble.child(arrow(side, t.primary));
            }
            let faded = bubble.with_animation(
                "shadcn-tooltip-fade",
                Animation::new(Duration::from_millis(150)).with_easing(gpui_kit::ease_out_quint()),
                |el, delta| el.opacity(delta),
            );
            Floating::new(anchor, measured, Placement::new(side, self.align, self.side_offset, 0.0), faded)
                .priority(TOOLTIP_PRIORITY)
                .occlude(false)
        });

        let hover_state = state.clone();
        let press_state = state.clone();
        div()
            .id(self.id)
            .merge_style(&self.style)
            .children(self.children)
            .child(track_bounds(state))
            .on_hover(move |hovered, _, cx| set(&hover_state, *hovered, cx))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                press_state.update(cx, |s, cx| {
                    if s.open {
                        s.open = false;
                        cx.notify();
                    }
                });
            })
            .children(body)
    }
}
