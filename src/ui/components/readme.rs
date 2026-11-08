//! `Markdown` (GitHub READMEs, release notes, AI chat messages) and `Readme` (an addon's GitHub README or changelog).
//!
//! Own renderer on top of `markdown_ast` (markdown-rs + html5ever + sanitizer) with three styles:
//! * [`MarkdownStyle::Readme`]: addon README and changelog text. Headings have a bottom border, paragraphs 8 px margins.
//! * [`MarkdownStyle::ReleaseNotes`]: release notes of addon updates. When the text has headings, a timeline rail with
//!   a dot per heading runs down the left side.
//! * [`MarkdownStyle::Chat`]: assistant messages in the AI chat.
//!
//! Parsing happens once per content change, and the result is cached in a keyed `Entity`. Small documents (such as
//! streaming chat messages) are parsed synchronously. Big ones are parsed on the background executor while the
//! previous version stays on screen.
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::{IconNamed, StyledExt, WindowExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, AppContext, Div, ElementId, Entity, EntityId, Font, FontStyle, FontWeight, Hsla,
    InteractiveElement, InteractiveText, IntoElement, ParentElement, RenderOnce, Resource, SharedString, SharedUri,
    StatefulInteractiveElement, StrikethroughStyle, StyleRefinement, Styled, StyledText, TextRun, UnderlineStyle,
    Window, div, img, px, relative, svg,
};
use gpui_kit::{ImageSource, ImgResourceLoader};

use super::external_link_dialog;
use super::markdown_ast::{
    self, Align, BOLD, Block, CODE, Cell as TableCell, Dim, Doc, Flow, ITALIC, ImageFlow, ListItem, MARK, ParseOptions,
    STRIKE, TextBlock, UNDERLINE,
};
pub use super::markdown_html::sanitize_markdown_html;
use super::markdown_http;
use super::scroll_area::{ScrollArea, ScrollbarAxis};
use crate::state::run_blocking;
use crate::ui::icons;
use crate::ui::theme::{FONT_MONO, Tokens, TokensExt, Tw, TwText, alpha, radius, tw};

/// Sources up to this many bytes are parsed on the UI thread (sub-millisecond in release builds); bigger ones in the
/// background.
const SYNC_PARSE_LIMIT: usize = 12 * 1024;

/// How the rendered markdown is styled.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum MarkdownStyle {
    /// Addon README and changelog text shown in dialogs. Text size and colour come from the surrounding text style.
    #[default]
    Readme,
    /// Release notes of addon updates: 14 px muted text, 12 px upper-case headings, and a timeline rail with a dot
    /// per heading when the text has headings.
    ReleaseNotes,
    /// Assistant messages in the AI chat, in small text.
    Chat,
}

type LinkHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

// =========================================================================================================
// Public component: Markdown
// =========================================================================================================

/// Markdown renderer. `Markdown::new("id", text).style(MarkdownStyle::Chat)`.
///
/// * `id`: must be unique and stable per instance (parse cache + hover/details state are keyed on it).
/// * `.base_url(..)`: relative image urls (and links, unless `.link_base_url` is set) resolve against it.
/// * `.on_link_click(|url, window, cx| ..)`: overrides the default (http(s) links open in the browser after the
///   [`external_link_dialog`](super::external_link_dialog) warning, mailto links open directly).
/// * `.timeline(bool)`: `ReleaseNotes` only, default = "text has headings". The rail assumes the caller adds
///   `.p_4().pb_8()` (16 px padding, 32 px at the bottom).
/// * Implements `Styled` (`.mt_2()`, `.p_4()`, `.text_sm_tw()`, ...), so callers can adjust spacing and text size.
/// * Text cannot be selected because GPUI has no cross-element selection. Expose a copy button with the source instead.
#[derive(IntoElement)]
pub struct Markdown {
    id: ElementId,
    source: SharedString,
    style: MarkdownStyle,
    image_base: Option<SharedString>,
    link_base: Option<SharedString>,
    on_link: Option<LinkHandler>,
    timeline: Option<bool>,
    refine: StyleRefinement,
}

impl Markdown {
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            style: MarkdownStyle::default(),
            image_base: None,
            link_base: None,
            on_link: None,
            timeline: None,
            refine: StyleRefinement::default(),
        }
    }

    pub fn style(mut self, style: MarkdownStyle) -> Self {
        self.style = style;
        self
    }

    /// Base url for relative images/links, e.g. `https://raw.githubusercontent.com/user/repo/main/`.
    pub fn base_url(mut self, url: impl Into<SharedString>) -> Self {
        self.image_base = Some(url.into());
        self
    }

    /// Separate base for relative *links* (e.g. `https://github.com/user/repo/blob/main/`); defaults to `base_url`.
    pub fn link_base_url(mut self, url: impl Into<SharedString>) -> Self {
        self.link_base = Some(url.into());
        self
    }

    pub fn on_link_click(mut self, handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_link = Some(Rc::new(handler));
        self
    }

    /// Force the release-notes rail on/off (default: on when the document has headings).
    pub fn timeline(mut self, on: bool) -> Self {
        self.timeline = Some(on);
        self
    }
}

impl Styled for Markdown {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.refine
    }
}

// --------------------------------------------------------------------------------------------------------
// Parse cache
// --------------------------------------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct ParseKey {
    source: SharedString,
    style: MarkdownStyle,
    image_base: Option<SharedString>,
    link_base: Option<SharedString>,
}

impl ParseKey {
    fn options(&self) -> ParseOptions {
        ParseOptions {
            breaks: self.style == MarkdownStyle::Chat,
            uppercase_headings: self.style == MarkdownStyle::ReleaseNotes,
            ..Default::default()
        }
        .with_bases(
            self.image_base.as_ref().map(|s| s.as_str()),
            self.link_base.as_ref().or(self.image_base.as_ref()).map(|s| s.as_str()),
        )
    }
}

#[derive(Default)]
struct MarkdownState {
    doc: Option<Arc<Doc>>,
    /// Key `doc` was parsed for.
    parsed: Option<ParseKey>,
    /// Latest requested key.
    want: Option<ParseKey>,
    pending: bool,
}

fn ensure_parsed(state: &Entity<MarkdownState>, key: ParseKey, window: &mut Window, cx: &mut App) {
    {
        let s = state.read(cx);
        if s.parsed.as_ref() == Some(&key) {
            return;
        }
        if s.want.as_ref() == Some(&key) && s.pending {
            return;
        }
    }
    if key.source.len() <= SYNC_PARSE_LIMIT {
        let doc = Arc::new(markdown_ast::parse(&key.source, &key.options()));
        state.update(cx, |s, _| {
            s.doc = Some(doc);
            s.parsed = Some(key.clone());
            s.want = Some(key);
        });
        return;
    }
    state.update(cx, |s, _| s.want = Some(key));
    if state.read(cx).pending {
        return; // the running task picks up the newer key when it finishes
    }
    state.update(cx, |s, _| s.pending = true);
    let view = window.current_view();
    let state = state.clone();
    window
        .spawn(cx, async move |cx| {
            loop {
                let Some(want) = state.read_with(cx, |s, _| s.want.clone()) else { return };
                let (src, opts) = (want.source.clone(), want.options());
                let doc = run_blocking(cx, move || markdown_ast::parse(&src, &opts)).await;
                let finished = state.update(cx, |s, _| {
                    s.doc = Some(Arc::new(doc));
                    s.parsed = Some(want.clone());
                    let finished = s.want.as_ref() == Some(&want);
                    if finished {
                        s.pending = false;
                    }
                    finished
                });
                cx.update(|_, cx| cx.notify(view)).ok();
                if finished {
                    return;
                }
            }
        })
        .detach();
}

// --------------------------------------------------------------------------------------------------------
// Look: per-style colours + metrics
// --------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Look {
    style: MarkdownStyle,
    t: Tokens,
    link: Hsla,
    link_hover: Hsla,
    strong: Option<Hsla>,
    strong_weight: FontWeight,
    em: Option<Hsla>,
    code_bg: Hsla,
    marker: Option<Hsla>,
    /// Colour of text inside `pre` blocks.
    pre_fg: Hsla,
    pre_bg: Hsla,
    pre_border: bool,
    image_scale: f32,
}

impl Look {
    fn new(style: MarkdownStyle, t: Tokens) -> Self {
        match style {
            MarkdownStyle::Readme | MarkdownStyle::ReleaseNotes => Self {
                style,
                t,
                link: t.primary,
                link_hover: alpha(t.primary, 0.8),
                strong: (style == MarkdownStyle::ReleaseNotes).then_some(t.foreground),
                strong_weight: FontWeight::BOLD,
                em: None,
                code_bg: t.muted,
                marker: (style == MarkdownStyle::ReleaseNotes).then_some(alpha(t.primary, 0.6)),
                pre_fg: t.muted_foreground,
                pre_bg: t.muted,
                pre_border: false,
                image_scale: 0.9,
            },
            MarkdownStyle::Chat => Self {
                style,
                t,
                link: tw(Tw::Blue, 400),
                link_hover: tw(Tw::Blue, 500),
                strong: Some(t.foreground),
                strong_weight: FontWeight::SEMIBOLD,
                em: Some(t.muted_foreground),
                code_bg: tw(Tw::Gray, 700),
                marker: None,
                pre_fg: t.muted_foreground,
                pre_bg: tw(Tw::Neutral, 900),
                pre_border: true,
                image_scale: 1.0,
            },
        }
    }
}

/// Text properties that nested elements inherit from their parent, passed down while rendering.
#[derive(Clone, Copy)]
struct Tcx {
    color: Hsla,
    weight: FontWeight,
    italic: bool,
}

struct Ctx {
    id: SharedString,
    n: Cell<usize>,
    look: Look,
    font: Font,
    on_link: LinkHandler,
    view: EntityId,
    /// Release notes rail on: headings get a dot.
    timeline: bool,
}

impl Ctx {
    fn next_id(&self, kind: &str) -> ElementId {
        let n = self.n.get();
        self.n.set(n + 1);
        ElementId::Name(format!("{}/{kind}{n}", self.id).into())
    }
}

fn default_link_handler() -> LinkHandler {
    Rc::new(open_link)
}

/// `http(s)` links open in the browser once the user proceeds through the leaving-the-app warning; `mailto:` links open
/// in the OS handler directly; `#anchors` and other schemes are ignored.
fn open_link(url: &SharedString, window: &mut Window, cx: &mut App) {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        external_link_dialog::open(window, cx, url.clone());
    } else if lower.starts_with("mailto:") {
        cx.open_url(url);
    }
}

// =========================================================================================================
// Render
// =========================================================================================================

impl RenderOnce for Markdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        markdown_http::install(cx);
        let t = cx.tokens();
        let state =
            window.use_keyed_state(SharedString::from(format!("{}/md", self.id)), cx, |_, _| MarkdownState::default());
        ensure_parsed(
            &state,
            ParseKey {
                source: self.source.clone(),
                style: self.style,
                image_base: self.image_base.clone(),
                link_base: self.link_base.clone(),
            },
            window,
            cx,
        );
        let doc = state.read(cx).doc.clone();

        let look = Look::new(self.style, t);
        let text_style = window.text_style();
        let base_color = match self.style {
            MarkdownStyle::Readme => text_style.color,
            MarkdownStyle::ReleaseNotes => t.muted_foreground,
            MarkdownStyle::Chat => t.foreground,
        };
        let timeline = self.style == MarkdownStyle::ReleaseNotes
            && self.timeline.unwrap_or_else(|| doc.as_ref().is_some_and(|d| d.has_headings));
        let ctx = Ctx {
            id: SharedString::from(format!("{}", self.id)),
            n: Cell::new(0),
            look,
            font: text_style.font(),
            on_link: self.on_link.clone().unwrap_or_else(default_link_handler),
            view: window.current_view(),
            timeline,
        };
        let tcx = Tcx { color: base_color, weight: FontWeight::NORMAL, italic: false };

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_col()
            .min_w_0()
            .when(self.style == MarkdownStyle::Chat, |d| d.text_sm_tw().text_color(t.foreground))
            .when(self.style == MarkdownStyle::ReleaseNotes, |d| {
                d.text_size(px(14.0)).line_height(relative(1.625)).text_color(t.muted_foreground)
            })
            .refine_style(&self.refine)
            .when(timeline, |d| d.pl(px(24.0)));

        if let Some(doc) = doc {
            let strip_top = self.style == MarkdownStyle::ReleaseNotes;
            for el in render_blocks(&doc.blocks, &ctx, tcx, strip_top, false, 0, window, cx) {
                root = root.child(el);
            }
        }
        if timeline {
            // Timeline rail: a 1 px line, 9 px from the left edge, from 24 px below the top to 32 px above the bottom.
            root = root.child(
                div().absolute().top(px(24.0)).bottom(px(32.0)).left(px(9.0)).w(px(1.0)).bg(alpha(t.primary, 0.3)),
            );
        }
        root
    }
}

// --------------------------------------------------------------------------------------------------------
// Blocks
// --------------------------------------------------------------------------------------------------------

/// (margin-top, margin-bottom) of a block, in px.
fn margin(block: &Block, style: MarkdownStyle, depth: usize, is_last: bool) -> (f32, f32) {
    use MarkdownStyle::*;
    match block {
        Block::Heading { .. } => match style {
            Readme => (8.0, 8.0),
            ReleaseNotes => (20.0, 8.0),
            Chat => (16.0, 8.0),
        },
        Block::Paragraph { implicit: true, .. } => (0.0, 0.0),
        Block::Paragraph { .. } => match style {
            Readme | ReleaseNotes => (8.0, 8.0),
            Chat => (0.0, if is_last { 0.0 } else { 12.0 }),
        },
        Block::Code { .. } => (16.0, 16.0),
        Block::Quote(_) => match style {
            Chat => (12.0, 12.0),
            _ => (8.0, 8.0),
        },
        Block::List { .. } => {
            if depth > 0 {
                return (4.0, 4.0);
            }
            match style {
                Readme => (20.0, 20.0),
                ReleaseNotes => (8.0, 8.0),
                Chat => (0.0, 12.0),
            }
        }
        Block::Table { .. } => (12.0, 12.0),
        Block::Rule => (16.0, 16.0),
        Block::Details { .. } => (8.0, 8.0),
    }
}

/// Renders `blocks` with collapsed margins: the gap between two neighbouring blocks is the larger of the previous
/// block's bottom margin and the next block's top margin, not their sum. `strip_top`/`strip_bottom` drop the outer
/// margins (children of list items, quotes, cells).
// Five of the arguments are the render context every block renderer threads through its recursion.
#[expect(clippy::too_many_arguments)]
fn render_blocks(
    blocks: &[Block],
    ctx: &Ctx,
    tcx: Tcx,
    strip_top: bool,
    strip_bottom: bool,
    depth: usize,
    window: &mut Window,
    cx: &mut App,
) -> Vec<Div> {
    let mut out = Vec::with_capacity(blocks.len());
    let mut prev_bottom = 0.0f32;
    for (i, block) in blocks.iter().enumerate() {
        let is_last = i + 1 == blocks.len();
        let (mt, mb) = margin(block, ctx.look.style, depth, is_last);
        let top = if i == 0 { if strip_top { 0.0 } else { mt } } else { prev_bottom.max(mt) };
        let bottom = if is_last && strip_bottom { 0.0 } else { mb };
        prev_bottom = mb;
        let el = render_block(block, ctx, tcx, depth, window, cx);
        out.push(el.mt(px(top)).when(is_last, |d| d.mb(px(bottom))));
    }
    out
}

fn render_block(block: &Block, ctx: &Ctx, tcx: Tcx, depth: usize, window: &mut Window, cx: &mut App) -> Div {
    let look = ctx.look;
    let t = look.t;
    match block {
        Block::Heading { level, flows, align } => render_heading(*level, flows, *align, ctx, tcx, window, cx),
        Block::Paragraph { flows, align, .. } => {
            let d = flows_el(flows, *align, tcx, ctx, window, cx);
            match look.style {
                MarkdownStyle::Chat => d.line_height(relative(1.625)),
                _ => d,
            }
        }
        Block::Code { text, .. } => {
            let tc = look.pre_fg;
            let scroll_id = ctx.next_id("pre");
            div()
                .w_full()
                .min_w_0()
                .rounded(radius::MD)
                .bg(look.pre_bg)
                .when(look.pre_border, |d| d.border_1().border_color(t.border))
                .child(
                    ScrollArea::new(scroll_id).axis(ScrollbarAxis::Horizontal).w_full().h_auto().child(
                        div()
                            .p_4()
                            .font_family(FONT_MONO)
                            .text_color(tc)
                            .when(look.style == MarkdownStyle::Chat, |d| d.text_sm_tw())
                            .whitespace_nowrap()
                            .child(text.clone()),
                    ),
                )
        }
        Block::Quote(blocks) => {
            let chat = look.style == MarkdownStyle::Chat;
            let inner_tcx = Tcx { color: t.muted_foreground, weight: tcx.weight, italic: chat };
            let mut d = div().flex().flex_col().w_full();
            d = if chat {
                d.border_l_4()
                    .border_color(alpha(t.primary, 0.3))
                    .pl_4()
                    .py_2()
                    .bg(alpha(t.muted, 0.3))
                    .rounded_r(px(4.0))
            } else {
                d.border_l_2().border_color(t.border).pl(px(12.0))
            };
            for el in render_blocks(blocks, ctx, inner_tcx, true, true, depth + 1, window, cx) {
                d = d.child(el);
            }
            d
        }
        Block::List { ordered, start, items } => {
            render_list(ordered.then_some(*start), items, ctx, tcx, depth, window, cx)
        }
        Block::Table { rows, weights } => render_table(rows, weights, ctx, tcx, depth, window, cx),
        Block::Rule => div().w_full().h(px(1.0)).bg(t.border),
        Block::Details { summary, open, blocks } => {
            let state_key = SharedString::from(format!("{}/open", ctx.next_id("details")));
            let state = window.use_keyed_state(state_key, cx, |_, _| *open);
            let is_open = *state.read(cx);
            let view = ctx.view;
            let sum = flows_el(summary, Align::Start, tcx, ctx, window, cx);
            let mut d = div().w_full().flex().flex_col();
            d = d.child(
                div()
                    .id(ctx.next_id("summary"))
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(4.0))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        state.update(cx, |s, _| *s = !*s);
                        cx.notify(view);
                        window.refresh();
                    })
                    .child(
                        div().flex_none().h(px(20.0)).flex().items_center().child(
                            svg()
                                .path(if is_open { icons::CHEVRON_DOWN.path() } else { icons::CHEVRON_RIGHT.path() })
                                .size(px(14.0))
                                .text_color(t.muted_foreground),
                        ),
                    )
                    .child(sum.flex_1().min_w_0()),
            );
            if is_open {
                let mut body = div().flex().flex_col().pl(px(18.0));
                for el in render_blocks(blocks, ctx, tcx, false, false, depth + 1, window, cx) {
                    body = body.child(el);
                }
                d = d.child(body);
            }
            d
        }
    }
}

/// (font size, line height, bold) of a heading.
fn heading_metrics(style: MarkdownStyle, level: u8) -> (Option<f32>, Option<f32>, bool) {
    match style {
        MarkdownStyle::ReleaseNotes => (Some(12.0), Some(16.0), false),
        MarkdownStyle::Chat => match level {
            1 => (Some(20.0), Some(28.0), false),
            2 => (Some(18.0), Some(28.0), false),
            3 => (Some(16.0), Some(24.0), false),
            _ => (None, None, false),
        },
        MarkdownStyle::Readme => match level {
            1 => (Some(30.0), Some(36.0), false),
            2 => (Some(24.0), Some(32.0), false),
            3 => (Some(20.0), Some(28.0), false),
            4 => (Some(18.0), Some(28.0), false),
            5 => (Some(16.0), Some(24.0), false),
            _ => (Some(14.0), Some(20.0), true),
        },
    }
}

fn render_heading(
    level: u8,
    flows: &[Flow],
    align: Align,
    ctx: &Ctx,
    tcx: Tcx,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let look = ctx.look;
    let t = look.t;
    let (size, lh, bold) = heading_metrics(look.style, level);
    let weight = if bold { FontWeight::BOLD } else { FontWeight::SEMIBOLD };
    let color = match look.style {
        MarkdownStyle::Readme => tcx.color,
        _ => t.foreground,
    };
    let inner = Tcx { color, weight, italic: tcx.italic };
    let content = flows_el(flows, align, inner, ctx, window, cx);
    let mut d = div()
        .relative()
        .w_full()
        .font_weight(weight)
        .text_color(color)
        .when_some(size, |d, s| d.text_size(px(s)))
        .when_some(lh, |d, l| d.line_height(px(l)))
        .child(content);
    match look.style {
        MarkdownStyle::Readme => {
            d = d.border_b_1().border_color(tw(Tw::Gray, 600)).pb(px(8.0));
        }
        MarkdownStyle::ReleaseNotes if ctx.timeline => {
            // Timeline dot: an 8 px circle on the rail, to the left of the heading text.
            d = d.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(-19.0))
                    .flex()
                    .items_center()
                    .child(div().size(px(8.0)).rounded_full().bg(t.primary)),
            );
        }
        MarkdownStyle::ReleaseNotes | MarkdownStyle::Chat => {}
    }
    d
}

// --------------------------------------------------------------------------------------------------------
// Lists
// --------------------------------------------------------------------------------------------------------

/// `start` is the first number of an ordered list, `None` for a bullet list.
fn render_list(
    start: Option<i64>,
    items: &[ListItem],
    ctx: &Ctx,
    tcx: Tcx,
    depth: usize,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let look = ctx.look;
    let t = look.t;
    let indent = if look.style == MarkdownStyle::Chat { 16.0 } else { 20.0 };
    let marker_color = look.marker.unwrap_or(tcx.color);
    let mut list = div().w_full().flex().flex_col().gap(px(4.0));
    for (i, item) in items.iter().enumerate() {
        let marker = if let Some(checked) = item.checked {
            // GFM task item: checkbox instead of the bullet.
            let boxed = div()
                .size(px(14.0))
                .flex_none()
                .rounded(px(4.0))
                .border_1()
                .border_color(if checked { t.primary } else { t.input })
                .when(checked, |d| d.bg(t.primary))
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |d| {
                    d.child(svg().path(icons::CHECK.path()).size(px(11.0)).text_color(t.primary_foreground))
                });
            div()
                .relative()
                .w(px(indent))
                .flex_none()
                .child("\u{200b}")
                .child(div().absolute().top_0().bottom_0().left_0().flex().items_center().child(boxed))
        } else if let Some(start) = start {
            div()
                .w(px(indent + 4.0))
                .flex_none()
                .text_right()
                .pr(px(6.0))
                .text_color(marker_color)
                .child(format!("{}.", start + i as i64))
        } else {
            let shape = match depth % 3 {
                0 => div().size(px(5.0)).rounded_full().bg(marker_color),
                1 => div().size(px(5.0)).rounded_full().border_1().border_color(marker_color),
                _ => div().size(px(4.0)).bg(marker_color),
            };
            div()
                .relative()
                .w(px(indent))
                .flex_none()
                .child("\u{200b}")
                .child(div().absolute().top_0().bottom_0().right_0().pr(px(7.0)).flex().items_center().child(shape))
        };
        let mut content = div().flex_1().min_w_0().flex().flex_col();
        for el in render_blocks(&item.blocks, ctx, tcx, true, true, depth + 1, window, cx) {
            content = content.child(el);
        }
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .when(start.is_some() && indent > 0.0, |d| d.ml(px(-4.0)))
                .child(marker)
                .child(content),
        );
    }
    list
}

// --------------------------------------------------------------------------------------------------------
// Tables
// --------------------------------------------------------------------------------------------------------

fn render_table(
    rows: &[Vec<TableCell>],
    weights: &[f32],
    ctx: &Ctx,
    tcx: Tcx,
    depth: usize,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let t = ctx.look.t;
    let mut table = div().w_full().min_w_0().flex().flex_col().border_1().border_color(t.border);
    for (ri, row) in rows.iter().enumerate() {
        let mut r =
            div().w_full().flex().flex_row().when(ri + 1 < rows.len(), |d| d.border_b_1().border_color(t.border));
        let mut col = 0;
        for (ci, cell) in row.iter().enumerate() {
            let w: f32 = weights.iter().skip(col).take(cell.colspan).sum::<f32>().max(1.0);
            col += cell.colspan;
            let inner = Tcx { weight: if cell.header { FontWeight::SEMIBOLD } else { tcx.weight }, ..tcx };
            let mut c = div()
                .flex()
                .flex_col()
                .flex_basis(px(0.0))
                .min_w_0()
                .px_3()
                .py_2()
                .when(cell.header, |d| d.bg(t.muted))
                .when(ci + 1 < row.len(), |d| d.border_r_1().border_color(t.border))
                .when(cell.align == Align::Center, |d| d.text_center())
                .when(cell.align == Align::End, |d| d.text_right());
            c.style().flex_grow = Some(w);
            c.style().flex_shrink = Some(1.0);
            for el in render_blocks(&cell.blocks, ctx, inner, true, true, depth + 1, window, cx) {
                c = c.child(el);
            }
            r = r.child(c);
        }
        table = table.child(r);
    }
    table
}

// --------------------------------------------------------------------------------------------------------
// Inline flows (text + images)
// --------------------------------------------------------------------------------------------------------

fn flows_el(flows: &[Flow], align: Align, tcx: Tcx, ctx: &Ctx, window: &mut Window, cx: &mut App) -> Div {
    let has_image = flows.iter().any(|f| matches!(f, Flow::Image(_)));
    if !has_image {
        let mut d = div().w_full();
        for f in flows {
            if let Flow::Text(tb) = f {
                d = d.child(text_el(tb, tcx, ctx, window, cx));
            }
        }
        return d.when(align == Align::Center, |d| d.text_center()).when(align == Align::End, |d| d.text_right());
    }
    let mut row = div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_x(px(4.0))
        .when(align == Align::Center, |d| d.justify_center().text_center())
        .when(align == Align::End, |d| d.justify_end().text_right());
    for f in flows {
        row = match f {
            Flow::Text(tb) => row.child(div().min_w_0().child(text_el(tb, tcx, ctx, window, cx))),
            Flow::Image(im) => row.child(image_el(im, ctx, window, cx)),
        };
    }
    row
}

fn text_el(tb: &TextBlock, tcx: Tcx, ctx: &Ctx, window: &mut Window, cx: &mut App) -> AnyElement {
    let look = ctx.look;
    let id = ctx.next_id("t");
    let links: Vec<(Range<usize>, SharedString)> =
        tb.spans.iter().filter_map(|s| s.link.clone().map(|l| (s.range.clone(), l))).collect();

    let hover_state = (!links.is_empty())
        .then(|| window.use_keyed_state(SharedString::from(format!("{id:?}/hover")), cx, |_, _| None::<usize>));
    let hovered = hover_state.as_ref().and_then(|s| *s.read(cx));

    let mut link_no = 0usize;
    let mut runs: Vec<TextRun> = Vec::with_capacity(tb.spans.len());
    for span in &tb.spans {
        let mut font = ctx.font.clone();
        font.weight = tcx.weight;
        let mut color = tcx.color;
        if span.flags & BOLD != 0 {
            font.weight = if tcx.weight > look.strong_weight { tcx.weight } else { look.strong_weight };
            if let Some(c) = look.strong {
                color = c;
            }
        }
        if span.flags & ITALIC != 0 || tcx.italic {
            font.style = FontStyle::Italic;
            if span.flags & ITALIC != 0
                && let Some(c) = look.em
            {
                color = c;
            }
        }
        let mut background = None;
        if span.flags & CODE != 0 {
            font.family = FONT_MONO.into();
            background = Some(look.code_bg);
            if span.flags & BOLD == 0 {
                font.weight = tcx.weight;
            }
        }
        if span.flags & MARK != 0 {
            background = Some(alpha(tw(Tw::Yellow, 300), 0.35));
        }
        let mut underline =
            (span.flags & UNDERLINE != 0).then_some(UnderlineStyle { thickness: px(1.0), color: None, wavy: false });
        if span.link.is_some() {
            let is_hover = hovered == Some(link_no);
            link_no += 1;
            color = if is_hover { look.link_hover } else { look.link };
            if is_hover {
                underline = Some(UnderlineStyle { thickness: px(1.0), color: None, wavy: false });
            }
        }
        runs.push(TextRun {
            len: span.range.len(),
            font,
            color,
            background_color: background,
            underline,
            strikethrough: (span.flags & STRIKE != 0).then_some(StrikethroughStyle { thickness: px(1.0), color: None }),
        });
    }
    let styled = StyledText::new(tb.text.clone()).with_runs(runs);

    let Some(state) = hover_state else { return styled.into_any_element() };
    let ranges: Vec<Range<usize>> = links.iter().map(|(r, _)| r.clone()).collect();
    let urls: Vec<SharedString> = links.into_iter().map(|(_, u)| u).collect();
    let on_link = ctx.on_link.clone();
    let view = ctx.view;
    let hover_ranges = ranges.clone();
    let hover_state = state.clone();
    let text = InteractiveText::new(id.clone(), styled)
        .on_click(ranges, move |ix, window, cx| on_link(&urls[ix], window, cx))
        .on_hover(move |ix, _, _, cx| {
            let pos = ix.and_then(|ix| hover_ranges.iter().position(|r| r.contains(&ix)));
            hover_state.update(cx, |s, _| *s = pos);
        });
    // Leaving the block must clear the hover underline (InteractiveText only reports moves over its text).
    div()
        .id(ElementId::Name(format!("{id:?}/wrap").into()))
        .on_hover(move |hovered, window, cx| {
            if !*hovered {
                let changed = state.update(cx, |s, _| s.take().is_some());
                if changed {
                    cx.notify(view);
                    window.refresh();
                }
            }
        })
        .child(text)
        .into_any_element()
}

// --------------------------------------------------------------------------------------------------------
// Images
// --------------------------------------------------------------------------------------------------------

/// Hosts that serve SVG without a `.svg` suffix (gpui rasterises svgs at 2x, so we have to halve their size).
fn is_svg_url(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    path.ends_with(".svg")
        || [
            "://img.shields.io/",
            "://badgen.net/",
            "://badge.fury.io/",
            "://flat.badgen.net/",
            "://api.codacy.com/project/badge",
        ]
        .iter()
        .any(|h| path.contains(h))
}

fn image_el(im: &ImageFlow, ctx: &Ctx, window: &mut Window, cx: &mut App) -> AnyElement {
    let look = ctx.look;
    let t = look.t;
    let resource = Resource::Uri(SharedUri::from(im.src.to_string()));
    let loaded = window.use_asset::<ImgResourceLoader>(&resource, cx);
    let margin_y = if look.style == MarkdownStyle::Chat { 0.0 } else { 12.0 };

    // (natural w, natural h) from the decoded bitmap.
    let natural = match &loaded {
        Some(Ok(data)) => {
            let s = data.size(0);
            let k = if is_svg_url(&im.src) { 2.0 } else { 1.0 };
            Some((s.width.0 as f32 / k, s.height.0 as f32 / k))
        }
        _ => None,
    };

    // Final box size: attribute dims win over the natural size, aspect ratio from the natural (or attr) size.
    let ratio = natural.map(|(w, h)| w / h.max(1.0));
    let (w, w_rel): (Option<f32>, Option<f32>) = match (im.width, im.height, natural) {
        (Some(Dim::Px(w)), _, _) => (Some(w), None),
        (Some(Dim::Percent(p)), _, _) => (None, Some(p)),
        (None, Some(Dim::Px(h)), Some(_)) => (Some(h * ratio.unwrap_or(1.0)), None),
        (None, _, Some((w, _))) => (Some(w * look.image_scale), None),
        _ => (None, None),
    };

    let alt_fallback = |alt: &SharedString| {
        div()
            .text_color(t.muted_foreground)
            .text_size(px(12.0))
            .italic()
            .child(if alt.is_empty() { SharedString::from("[image]") } else { alt.clone() })
            .into_any_element()
    };

    match loaded {
        Some(Ok(data)) => {
            let ratio = ratio.unwrap_or(1.0);
            let id = ctx.next_id("img");
            let link = im.link.clone();
            let on_link = ctx.on_link.clone();
            let (src, alt, nat) = (im.src.clone(), im.alt.clone(), natural.unwrap_or((0.0, 0.0)));
            let mut boxed = div()
                .id(id)
                .flex_none()
                .max_w_full()
                .my(px(margin_y))
                .cursor_pointer()
                .when_some(w, |d, w| d.w(px(w)))
                .when_some(w_rel, |d, p| d.w(relative(p)))
                .child(img(ImageSource::Render(data)).size_full())
                .on_click(move |_, window, cx| {
                    if let Some(link) = &link {
                        on_link(link, window, cx);
                    } else {
                        open_image_viewer(&src, &alt, nat, window, cx);
                    }
                });
            boxed.style().aspect_ratio = Some(ratio);
            boxed.into_any_element()
        }
        Some(Err(_)) => alt_fallback(&im.alt),
        None => {
            // Loading: reserve the space when the size is known from attributes.
            let mut d = div()
                .flex_none()
                .max_w_full()
                .my(px(margin_y))
                .rounded(radius::SM)
                .bg(alpha(t.muted, 0.4))
                .when_some(w, |d, w| d.w(px(w)))
                .when(w.is_none(), |d| d.w(px(56.0)).h(px(20.0)));
            if let (Some(Dim::Px(w)), Some(Dim::Px(h))) = (im.width, im.height) {
                d.style().aspect_ratio = Some(w / h);
            } else if w.is_some() {
                d.style().aspect_ratio = Some(2.0);
            }
            d.into_any_element()
        }
    }
}

/// Full-size image viewer dialog, opened by clicking an image that is not a link. `natural` = (w, h) in px or
/// `(0., 0.)` when unknown. Uses `Root`'s dialog layer (hosted by gpui-kit).
pub fn open_image_viewer(
    src: &SharedString,
    alt: &SharedString,
    natural: (f32, f32),
    window: &mut Window,
    cx: &mut App,
) {
    let vp = window.viewport_size();
    let (max_w, max_h) = (f32::from(vp.width) * 0.8, f32::from(vp.height) * 0.7);
    let (nw, nh) = if natural.0 > 0.0 { natural } else { (max_w, max_h) };
    let k = (max_w / nw).min(max_h / nh).clamp(0.05, 1.0);
    let (w, h) = (nw * k, nh * k);
    let (src, alt) = (src.clone(), alt.clone());
    super::dialog::open_layer(window, cx, move |dialog, _topmost, _, _| {
        dialog
            .w(px(w + 48.0))
            .p(px(24.0)) // 0.7 defaults to 16px; the viewer was laid out for 24px
            .title(alt.clone())
            .overlay_closable(true)
            .on_ok(|_, _, _| false) // Enter does nothing (0.7 would close the dialog)
            .child(div().w_full().flex().justify_center().child(img(src.to_string()).w(px(w)).h(px(h))))
    });
}

// =========================================================================================================
// Public component: Readme
// =========================================================================================================

/// Addon README / changelog: `Markdown` with `MarkdownStyle::Readme` and GitHub url resolution.
///
/// `Readme::new("id", text).repo("owner/name").branch("main")`: relative images resolve against
/// `https://raw.githubusercontent.com/{repo}/{branch}/` (branch defaults to `HEAD`), relative links against
/// `https://github.com/{repo}/blob/{branch}/`. Without `.repo` urls are left untouched. Text size and colour are
/// inherited, so wrap it in `.text_sm_tw()` etc. Implements `Styled`.
/// The loading skeleton and the "no README" fallback are the caller's job.
#[derive(IntoElement)]
pub struct Readme {
    markdown: Markdown,
    repo: Option<SharedString>,
    branch: Option<SharedString>,
}

impl Readme {
    pub fn new(id: impl Into<ElementId>, content: impl Into<SharedString>) -> Self {
        Self { markdown: Markdown::new(id, content), repo: None, branch: None }
    }

    /// `owner/name` of the GitHub repository (empty = none).
    pub fn repo(mut self, repo: impl Into<SharedString>) -> Self {
        let repo: SharedString = repo.into();
        self.repo = (!repo.trim().is_empty()).then_some(repo);
        self
    }

    pub fn branch(mut self, branch: impl Into<SharedString>) -> Self {
        let branch: SharedString = branch.into();
        self.branch = (!branch.trim().is_empty()).then_some(branch);
        self
    }

    pub fn on_link_click(mut self, handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.markdown = self.markdown.on_link_click(handler);
        self
    }
}

impl Styled for Readme {
    fn style(&mut self) -> &mut StyleRefinement {
        Styled::style(&mut self.markdown)
    }
}

impl RenderOnce for Readme {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut md = self.markdown.style(MarkdownStyle::Readme);
        if let Some(repo) = &self.repo {
            let branch = self.branch.as_ref().map_or("HEAD", |b| b.as_str());
            let repo = repo.as_str();
            md = md
                .base_url(format!("https://raw.githubusercontent.com/{repo}/{branch}/"))
                .link_base_url(format!("https://github.com/{repo}/blob/{branch}/"));
        }
        md
    }
}
