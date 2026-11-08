//! Markdown -> sanitised HTML -> DOM -> a small, `Send + Sync` document tree ([`Doc`]) that `readme.rs` renders.
//!
//! Pipeline (all on whatever thread calls [`parse`], the widget runs it on the background executor for big docs):
//! 1. `markdown` (markdown-rs, GFM, raw HTML allowed) renders the source to HTML.
//! 2. `markdown_html::sanitize_dom` strips everything dangerous (scripts, event handlers, unsafe urls ...).
//! 3. The DOM is folded into [`Block`]s / [`Flow`]s with browser-like whitespace collapsing, relative urls resolved.
extern crate markup5ever_rcdom as rcdom;

use std::ops::Range;

use gpui_kit::SharedString;
use rcdom::{Handle, NodeData};
use url::Url;

use super::markdown_html::{attr, parse_fragment, sanitize_dom};

// --------------------------------------------------------------------------------------------------------
// Tree
// --------------------------------------------------------------------------------------------------------

/// Parsed document.
#[derive(Debug, Default, PartialEq)]
pub struct Doc {
    pub blocks: Vec<Block>,
    /// Whether the document contains at least one heading. Release notes with headings get the timeline rail.
    pub has_headings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        flows: Vec<Flow>,
        align: Align,
    },
    /// `implicit`: loose inline content of a tight list item / table cell (no `<p>`), so no paragraph margins.
    Paragraph {
        flows: Vec<Flow>,
        align: Align,
        implicit: bool,
    },
    Code {
        lang: Option<String>,
        text: SharedString,
    },
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: i64,
        items: Vec<ListItem>,
    },
    Table {
        rows: Vec<Vec<Cell>>,
        weights: Vec<f32>,
    },
    Rule,
    Details {
        summary: Vec<Flow>,
        open: bool,
        blocks: Vec<Block>,
    },
}

#[derive(Debug, PartialEq)]
pub struct ListItem {
    /// `Some(checked)` for GFM task list items.
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, PartialEq)]
pub struct Cell {
    pub header: bool,
    pub align: Align,
    pub colspan: usize,
    pub blocks: Vec<Block>,
}

/// A piece of inline flow: a run of styled text or an image.
#[derive(Debug, PartialEq)]
pub enum Flow {
    Text(TextBlock),
    Image(ImageFlow),
}

#[derive(Debug, PartialEq)]
pub struct TextBlock {
    pub text: SharedString,
    /// Sorted, non-overlapping, covering `text` completely.
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub range: Range<usize>,
    pub flags: u8,
    pub link: Option<SharedString>,
}

pub const BOLD: u8 = 1;
pub const ITALIC: u8 = 2;
pub const STRIKE: u8 = 4;
pub const CODE: u8 = 8;
pub const UNDERLINE: u8 = 16;
pub const MARK: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dim {
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageFlow {
    pub src: SharedString,
    pub alt: SharedString,
    pub width: Option<Dim>,
    pub height: Option<Dim>,
    pub link: Option<SharedString>,
}

impl Flow {
    /// Plain text length (for table column weights).
    pub fn text_len(&self) -> usize {
        match self {
            Flow::Text(t) => t.text.chars().count(),
            Flow::Image(_) => 8,
        }
    }
}

impl TextBlock {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        let text: SharedString = text.into();
        let len = text.len();
        Self { text, spans: vec![Span { range: 0..len, flags: 0, link: None }] }
    }
}

// --------------------------------------------------------------------------------------------------------
// Options
// --------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    /// Single newlines become line breaks (used for chat messages).
    pub breaks: bool,
    /// Upper-case heading text (used for release notes).
    pub uppercase_headings: bool,
    /// Relative image urls resolve against this base (e.g. `https://raw.githubusercontent.com/user/repo/main/`).
    pub image_base: Option<Url>,
    /// Relative link urls resolve against this base (defaults to `image_base` when `None`).
    pub link_base: Option<Url>,
}

impl ParseOptions {
    pub fn with_bases(mut self, image_base: Option<&str>, link_base: Option<&str>) -> Self {
        self.image_base = image_base.and_then(|s| Url::parse(s).ok());
        self.link_base = link_base.and_then(|s| Url::parse(s).ok());
        self
    }
}

/// Resolves `raw` against `base`. Absolute urls and `#anchors` are returned unchanged.
fn resolve(base: Option<&Url>, raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('#') || Url::parse(raw).is_ok() {
        return raw.to_string();
    }
    let Some(base) = base else { return raw.to_string() };
    // `/docs/x.png` on github means "from the repo root", not from the host root.
    let rel = if raw.starts_with('/') && !raw.starts_with("//") { &raw[1..] } else { raw };
    base.join(rel).map(|u| u.to_string()).unwrap_or_else(|_| raw.to_string())
}

/// `https://github.com/u/r/blob/main/a/b.png?raw=true` -> `https://raw.githubusercontent.com/u/r/main/a/b.png`.
fn github_blob_to_raw(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else { return url.to_string() };
    if !matches!(parsed.host_str(), Some("github.com" | "www.github.com")) {
        return url.to_string();
    }
    let segs: Vec<&str> = parsed.path_segments().map(|s| s.collect()).unwrap_or_default();
    if segs.len() >= 5 && segs[2] == "blob" {
        return format!("https://raw.githubusercontent.com/{}/{}/{}", segs[0], segs[1], segs[3..].join("/"));
    }
    url.to_string()
}

// --------------------------------------------------------------------------------------------------------
// Parse
// --------------------------------------------------------------------------------------------------------

/// Parses markdown (with embedded HTML) into a [`Doc`].
pub fn parse(source: &str, opts: &ParseOptions) -> Doc {
    let mut md = markdown::Options::gfm();
    md.compile.allow_dangerous_html = true;
    md.compile.gfm_tagfilter = false;
    let html = markdown::to_html_with_options(source, &md).unwrap_or_else(|_| escape_html(source));
    parse_html(&html, opts)
}

/// Parses (and sanitises) an HTML fragment into a [`Doc`].
pub fn parse_html(html: &str, opts: &ParseOptions) -> Doc {
    let fragment = parse_fragment(html);
    sanitize_dom(fragment.root());
    let mut b = Builder { opts, has_headings: false, picture_src: None };
    let mut blocks = Vec::new();
    b.blocks_from_children(fragment.root(), Align::Start, &mut blocks);
    Doc { blocks, has_headings: b.has_headings }
}

fn escape_html(s: &str) -> String {
    format!("<p>{}</p>", s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"))
}

struct Builder<'a> {
    opts: &'a ParseOptions,
    has_headings: bool,
    /// `<picture><source media="(prefers-color-scheme: dark)" srcset=..>` override for the next `<img>`.
    picture_src: Option<String>,
}

fn children(node: &Handle) -> Vec<Handle> {
    node.children.borrow().clone()
}

fn tag(node: &Handle) -> Option<&str> {
    match &node.data {
        NodeData::Element { name, .. } => Some(&name.local),
        _ => None,
    }
}

fn is_block_tag(t: &str) -> bool {
    matches!(
        t,
        "p" | "div"
            | "section"
            | "article"
            | "aside"
            | "header"
            | "footer"
            | "main"
            | "nav"
            | "address"
            | "figure"
            | "figcaption"
            | "center"
            | "details"
            | "summary"
            | "blockquote"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "table"
            | "caption"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "hr"
            | "pre"
    )
}

fn parse_align(v: Option<String>) -> Option<Align> {
    match v?.trim().to_ascii_lowercase().as_str() {
        "center" | "middle" => Some(Align::Center),
        "right" | "end" => Some(Align::End),
        "left" | "start" => Some(Align::Start),
        _ => None,
    }
}

fn parse_dim(v: Option<String>) -> Option<Dim> {
    let v = v?;
    let v = v.trim();
    if let Some(p) = v.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().filter(|n| *n > 0.0).map(|n| Dim::Percent(n / 100.0));
    }
    v.trim_end_matches("px").trim().parse::<f32>().ok().filter(|n| *n > 0.0).map(Dim::Px)
}

fn text_content(node: &Handle, out: &mut String) {
    match &node.data {
        NodeData::Text { contents } => out.push_str(&contents.borrow()),
        NodeData::Element { name, .. } if &*name.local == "br" => out.push('\n'),
        _ => {
            for c in node.children.borrow().iter() {
                text_content(c, out);
            }
        }
    }
}

impl Builder<'_> {
    /// Folds the children of `parent` into blocks; stray inline content becomes implicit paragraphs.
    fn blocks_from_children(&mut self, parent: &Handle, align: Align, out: &mut Vec<Block>) {
        let mut inl = Inl::new(self.opts, false);
        for child in children(parent) {
            match &child.data {
                NodeData::Text { contents } => inl.push_text(&contents.borrow(), 0, &None),
                NodeData::Element { name, .. } => {
                    let t: &str = &name.local;
                    if is_block_tag(t) {
                        self.flush(&mut inl, align, out);
                        self.block_element(&child, t, align, out);
                    } else {
                        self.inline_element(&child, &mut inl, 0, &None);
                    }
                }
                _ => {}
            }
        }
        self.flush(&mut inl, align, out);
    }

    fn flush(&mut self, inl: &mut Inl, align: Align, out: &mut Vec<Block>) {
        let flows = std::mem::replace(inl, Inl::new(self.opts, false)).finish();
        if !flows.is_empty() {
            out.push(Block::Paragraph { flows, align, implicit: true });
        }
    }

    fn inline_flows(&mut self, node: &Handle, uppercase: bool) -> Vec<Flow> {
        let mut inl = Inl::new(self.opts, uppercase);
        for child in children(node) {
            self.inline_node(&child, &mut inl, 0, &None);
        }
        inl.finish()
    }

    fn inline_node(&mut self, node: &Handle, inl: &mut Inl, flags: u8, link: &Option<SharedString>) {
        match &node.data {
            NodeData::Text { contents } => inl.push_text(&contents.borrow(), flags, link),
            NodeData::Element { .. } => self.inline_element(node, inl, flags, link),
            _ => {}
        }
    }

    fn inline_element(&mut self, node: &Handle, inl: &mut Inl, flags: u8, link: &Option<SharedString>) {
        let Some(t) = tag(node) else { return };
        let t = t.to_string();
        match t.as_str() {
            "br" => inl.line_break(),
            "wbr" | "source" => {}
            "img" => {
                let src = self.picture_src.take().or_else(|| attr(node, "src")).unwrap_or_default();
                if src.trim().is_empty() {
                    return;
                }
                let src = github_blob_to_raw(&resolve(self.opts.image_base.as_ref(), &src));
                inl.image(ImageFlow {
                    src: src.into(),
                    alt: attr(node, "alt").unwrap_or_default().into(),
                    width: parse_dim(attr(node, "width")),
                    height: parse_dim(attr(node, "height")),
                    link: link.clone(),
                });
            }
            "picture" => {
                for c in children(node) {
                    if tag(&c) == Some("source") {
                        let dark =
                            attr(&c, "media").is_some_and(|m| m.replace(' ', "").contains("prefers-color-scheme:dark"));
                        if dark
                            && let Some(set) = attr(&c, "srcset")
                            && let Some(first) = set.split(',').next().and_then(|s| s.split_whitespace().next())
                        {
                            self.picture_src = Some(first.to_string());
                        }
                    }
                }
                for c in children(node) {
                    self.inline_node(&c, inl, flags, link);
                }
                self.picture_src = None;
            }
            "input" => {
                let checked = attr(node, "checked").is_some();
                inl.push_text(if checked { "\u{2611} " } else { "\u{2610} " }, flags, link);
            }
            "a" => {
                let href = attr(node, "href").filter(|h| !h.trim().is_empty()).map(|h| {
                    SharedString::from(resolve(self.opts.link_base.as_ref().or(self.opts.image_base.as_ref()), &h))
                });
                let link = href.or_else(|| link.clone());
                for c in children(node) {
                    self.inline_node(&c, inl, flags, &link);
                }
            }
            "code" | "kbd" | "samp" | "tt" => {
                inl.push_text_raw("\u{2009}", flags | CODE, link);
                for c in children(node) {
                    self.inline_node(&c, inl, flags | CODE, link);
                }
                inl.push_text_raw("\u{2009}", flags | CODE, link);
            }
            _ => {
                let f = match t.as_str() {
                    "strong" | "b" => flags | BOLD,
                    "em" | "i" | "cite" | "var" | "dfn" => flags | ITALIC,
                    "del" | "s" | "strike" => flags | STRIKE,
                    "u" | "ins" => flags | UNDERLINE,
                    "mark" => flags | MARK,
                    _ => flags,
                };
                for c in children(node) {
                    self.inline_node(&c, inl, f, link);
                }
            }
        }
    }

    fn block_element(&mut self, node: &Handle, t: &str, align: Align, out: &mut Vec<Block>) {
        let own_align = parse_align(attr(node, "align"));
        match t {
            "p" => {
                let flows = self.inline_flows(node, false);
                if !flows.is_empty() {
                    out.push(Block::Paragraph { flows, align: own_align.unwrap_or(align), implicit: false });
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.has_headings = true;
                let flows = self.inline_flows(node, self.opts.uppercase_headings);
                if !flows.is_empty() {
                    let level = t.as_bytes()[1] - b'0';
                    out.push(Block::Heading { level, flows, align: own_align.unwrap_or(align) });
                }
            }
            "pre" => {
                let lang =
                    children(node).iter().find(|c| tag(c) == Some("code")).and_then(|c| attr(c, "class")).and_then(
                        |c| c.split_whitespace().find_map(|k| k.strip_prefix("language-").map(str::to_string)),
                    );
                let mut text = String::new();
                text_content(node, &mut text);
                while text.ends_with('\n') {
                    text.pop();
                }
                out.push(Block::Code { lang, text: text.replace('\t', "    ").into() });
            }
            "blockquote" => {
                let mut inner = Vec::new();
                self.blocks_from_children(node, align, &mut inner);
                out.push(Block::Quote(inner));
            }
            "ul" | "ol" => {
                let start = attr(node, "start").and_then(|s| s.trim().parse().ok()).unwrap_or(1);
                let mut items = Vec::new();
                for li in children(node) {
                    if tag(&li) != Some("li") {
                        continue;
                    }
                    let checked = take_checkbox(&li);
                    let mut blocks = Vec::new();
                    self.blocks_from_children(&li, align, &mut blocks);
                    items.push(ListItem { checked, blocks });
                }
                out.push(Block::List { ordered: t == "ol", start, items });
            }
            "hr" => out.push(Block::Rule),
            "table" => {
                let mut rows = Vec::new();
                self.collect_rows(node, &mut rows);
                if !rows.is_empty() {
                    let ncols = rows.iter().map(|r| r.iter().map(|c| c.colspan).sum::<usize>()).max().unwrap_or(0);
                    let mut weights = vec![4.0f32; ncols];
                    for row in &rows {
                        let mut col = 0;
                        for cell in row {
                            let len = cell_text_len(&cell.blocks) as f32;
                            let per = (len / cell.colspan as f32).clamp(4.0, 60.0);
                            for w in weights.iter_mut().skip(col).take(cell.colspan) {
                                *w = w.max(per);
                            }
                            col += cell.colspan;
                        }
                    }
                    out.push(Block::Table { rows, weights });
                }
            }
            "details" => {
                let open = attr(node, "open").is_some();
                let mut summary = Vec::new();
                let mut inner = Vec::new();
                let kids = children(node);
                let mut inl = Inl::new(self.opts, false);
                for c in kids {
                    if tag(&c) == Some("summary") {
                        summary = self.inline_flows(&c, false);
                    } else {
                        match &c.data {
                            NodeData::Text { contents } => inl.push_text(&contents.borrow(), 0, &None),
                            NodeData::Element { name, .. } => {
                                let n: &str = &name.local;
                                if is_block_tag(n) {
                                    self.flush(&mut inl, align, &mut inner);
                                    self.block_element(&c, n, align, &mut inner);
                                } else {
                                    self.inline_element(&c, &mut inl, 0, &None);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                self.flush(&mut inl, align, &mut inner);
                if summary.is_empty() {
                    summary = vec![Flow::Text(TextBlock::plain("Details"))];
                }
                out.push(Block::Details { summary, open, blocks: inner });
            }
            // div, section, center, li outside of lists, dl/dd/dt, figure, thead/tr outside of tables ...
            _ => {
                let a = if t == "center" { Align::Center } else { own_align.unwrap_or(align) };
                self.blocks_from_children(node, a, out);
            }
        }
    }

    fn collect_rows(&mut self, node: &Handle, rows: &mut Vec<Vec<Cell>>) {
        for c in children(node) {
            match tag(&c) {
                Some("tr") => {
                    let mut cells = Vec::new();
                    for cell in children(&c) {
                        let Some(ct @ ("td" | "th")) = tag(&cell) else { continue };
                        let mut blocks = Vec::new();
                        self.blocks_from_children(&cell, Align::Start, &mut blocks);
                        cells.push(Cell {
                            header: ct == "th",
                            align: parse_align(attr(&cell, "align")).unwrap_or_default(),
                            colspan: attr(&cell, "colspan")
                                .and_then(|s| s.trim().parse().ok())
                                .filter(|n| (1..=50).contains(n))
                                .unwrap_or(1),
                            blocks,
                        });
                    }
                    if !cells.is_empty() {
                        rows.push(cells);
                    }
                }
                Some("thead" | "tbody" | "tfoot") => self.collect_rows(&c, rows),
                _ => {}
            }
        }
    }
}

fn cell_text_len(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph { flows, .. } | Block::Heading { flows, .. } => flows.iter().map(Flow::text_len).sum(),
            Block::Code { text, .. } => text.len(),
            Block::Quote(b) => cell_text_len(b),
            _ => 0,
        })
        .sum()
}

/// If the list item starts with a task-list checkbox, removes it and returns its state.
fn take_checkbox(li: &Handle) -> Option<bool> {
    fn first_meaningful(node: &Handle) -> Option<Handle> {
        node.children
            .borrow()
            .iter()
            .find(|c| match &c.data {
                NodeData::Text { contents } => !contents.borrow().trim().is_empty(),
                NodeData::Element { .. } => true,
                _ => false,
            })
            .cloned()
    }
    let first = first_meaningful(li)?;
    let target = match tag(&first) {
        Some("input") => (li.clone(), first),
        Some("p") => {
            let inner = first_meaningful(&first)?;
            if tag(&inner) != Some("input") {
                return None;
            }
            (first, inner)
        }
        _ => return None,
    };
    let (holder, input) = target;
    if attr(&input, "type").as_deref() != Some("checkbox") {
        return None;
    }
    let checked = attr(&input, "checked").is_some();
    holder.children.borrow_mut().retain(|c| !std::rc::Rc::ptr_eq(c, &input));
    Some(checked)
}

// --------------------------------------------------------------------------------------------------------
// Inline builder (HTML whitespace collapsing)
// --------------------------------------------------------------------------------------------------------

/// Makes `spans` a gapless, non-overlapping cover of `text` (the renderer's `StyledText::with_runs` panics
/// otherwise). Gaps become plain spans.
fn normalize_spans(text: &str, spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    let mut cursor = 0;
    for mut s in spans {
        let end = s.range.end.min(text.len());
        if s.range.start > cursor {
            out.push(Span { range: cursor..s.range.start.min(text.len()), flags: 0, link: None });
            cursor = s.range.start.min(text.len());
        }
        if end > cursor && text.is_char_boundary(cursor) && text.is_char_boundary(end) {
            s.range = cursor..end;
            cursor = end;
            out.push(s);
        }
    }
    if cursor < text.len() {
        out.push(Span { range: cursor..text.len(), flags: 0, link: None });
    }
    out
}

struct Inl {
    flows: Vec<Flow>,
    text: String,
    spans: Vec<Span>,
    last_space: bool,
    breaks: bool,
    uppercase: bool,
}

impl Inl {
    fn new(opts: &ParseOptions, uppercase: bool) -> Self {
        Self {
            flows: Vec::new(),
            text: String::new(),
            spans: Vec::new(),
            last_space: true,
            breaks: opts.breaks,
            uppercase,
        }
    }

    fn push_span(&mut self, start: usize, flags: u8, link: &Option<SharedString>) {
        let end = self.text.len();
        if end == start {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.range.end == start
            && last.flags == flags
            && last.link == *link
        {
            last.range.end = end;
            return;
        }
        self.spans.push(Span { range: start..end, flags, link: link.clone() });
    }

    /// Adds text without collapsing (used for the code-span padding).
    fn push_text_raw(&mut self, s: &str, flags: u8, link: &Option<SharedString>) {
        let start = self.text.len();
        self.text.push_str(s);
        self.push_span(start, flags, link);
        self.last_space = false;
    }

    fn push_text(&mut self, s: &str, flags: u8, link: &Option<SharedString>) {
        let start = self.text.len();
        let upper;
        let s = if self.uppercase {
            upper = s.to_uppercase();
            upper.as_str()
        } else {
            s
        };
        for ch in s.chars() {
            if ch == '\n' && self.breaks {
                self.trim_trailing_space();
                self.text.push('\n');
                self.last_space = true;
            } else if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
                if !self.last_space {
                    self.text.push(' ');
                    self.last_space = true;
                }
            } else {
                self.text.push(ch);
                self.last_space = false;
            }
        }
        // Span for what was appended (the start may sit after trimmed text, so clamp).
        let start = start.min(self.text.len());
        self.push_span(start, flags, link);
    }

    fn trim_trailing_space(&mut self) {
        if self.text.ends_with(' ') {
            self.text.pop();
            if let Some(last) = self.spans.last_mut() {
                if last.range.end > self.text.len() {
                    last.range.end = self.text.len();
                }
                if last.range.is_empty() {
                    self.spans.pop();
                }
            }
        }
    }

    fn line_break(&mut self) {
        self.trim_trailing_space();
        if !self.text.is_empty() || !self.flows.is_empty() {
            let start = self.text.len();
            self.text.push('\n');
            let (flags, link) = self.spans.last().map(|s| (s.flags & !CODE, s.link.clone())).unwrap_or((0, None));
            self.push_span(start, flags, &link);
        }
        self.last_space = true;
    }

    fn flush_text(&mut self) {
        // Drop trailing collapsible whitespace / breaks.
        while self.text.ends_with([' ', '\n']) {
            self.text.pop();
        }
        let len = self.text.len();
        self.spans.retain_mut(|s| {
            s.range.end = s.range.end.min(len);
            !s.range.is_empty()
        });
        let text = std::mem::take(&mut self.text);
        let spans = normalize_spans(&text, std::mem::take(&mut self.spans));
        if !text.trim().is_empty() {
            self.flows.push(Flow::Text(TextBlock { text: text.into(), spans }));
        }
    }

    fn image(&mut self, img: ImageFlow) {
        self.flush_text();
        self.flows.push(Flow::Image(img));
        self.last_space = true;
    }

    fn finish(mut self) -> Vec<Flow> {
        self.flush_text();
        self.flows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ParseOptions {
        ParseOptions::default()
            .with_bases(Some("https://raw.githubusercontent.com/u/r/main/"), Some("https://github.com/u/r/blob/main/"))
    }

    fn text_of(flows: &[Flow]) -> String {
        flows
            .iter()
            .map(|f| match f {
                Flow::Text(t) => t.text.to_string(),
                Flow::Image(i) => format!("[img {}]", i.src),
            })
            .collect()
    }

    #[test]
    fn paragraph_and_inline_marks() {
        let doc = parse("Hello **bold** and *it* [link](https://x.y) `code`", &opts());
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!("{doc:?}") };
        let Flow::Text(t) = &flows[0] else { panic!() };
        assert!(t.text.starts_with("Hello bold and it link"));
        assert!(t.spans.iter().any(|s| s.flags & BOLD != 0 && &t.text[s.range.clone()] == "bold"));
        assert!(t.spans.iter().any(|s| s.flags & ITALIC != 0));
        assert!(t.spans.iter().any(|s| s.link.as_ref().map(|l| l.as_str()) == Some("https://x.y")));
        assert!(t.spans.iter().any(|s| s.flags & CODE != 0));
        // spans cover the text completely
        assert_eq!(t.spans.first().unwrap().range.start, 0);
        assert_eq!(t.spans.last().unwrap().range.end, t.text.len());
    }

    #[test]
    fn heading_and_has_headings() {
        let doc = parse("# Title\n\ntext", &opts());
        assert!(doc.has_headings);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(!parse("just text", &opts()).has_headings);
    }

    #[test]
    fn uppercase_headings() {
        let mut o = opts();
        o.uppercase_headings = true;
        let doc = parse("## Bug fixes", &o);
        let Block::Heading { flows, .. } = &doc.blocks[0] else { panic!() };
        assert_eq!(text_of(flows), "BUG FIXES");
    }

    #[test]
    fn relative_image_and_link_urls() {
        let doc = parse("![a](img/a.png)\n\n[doc](docs/x.md) ![b](/root.png)", &opts());
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!() };
        assert_eq!(text_of(flows), "[img https://raw.githubusercontent.com/u/r/main/img/a.png]");
        let Block::Paragraph { flows, .. } = &doc.blocks[1] else { panic!() };
        let Flow::Text(t) = &flows[0] else { panic!() };
        assert_eq!(t.spans[0].link.as_ref().map(|l| l.as_str()), Some("https://github.com/u/r/blob/main/docs/x.md"));
        let Flow::Image(i) = &flows[1] else { panic!() };
        assert_eq!(i.src.as_str(), "https://raw.githubusercontent.com/u/r/main/root.png");
    }

    #[test]
    fn github_blob_images_become_raw() {
        let doc = parse("![a](https://github.com/u/r/blob/main/x/y.png?raw=true)", &opts());
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!() };
        assert_eq!(text_of(flows), "[img https://raw.githubusercontent.com/u/r/main/x/y.png]");
    }

    #[test]
    fn html_centered_images_and_links() {
        let doc =
            parse("<p align=\"center\"><a href=\"https://a.b\"><img src=\"logo.png\" width=\"200\"></a></p>", &opts());
        let Block::Paragraph { flows, align, .. } = &doc.blocks[0] else { panic!("{doc:?}") };
        assert_eq!(*align, Align::Center);
        let Flow::Image(i) = &flows[0] else { panic!() };
        assert_eq!(i.width, Some(Dim::Px(200.0)));
        assert_eq!(i.link.as_ref().map(|l| l.as_str()), Some("https://a.b"));
    }

    #[test]
    fn script_is_removed_but_code_fence_kept() {
        let doc = parse("<script>alert(1)</script>\n\nok\n\n```html\n<script>x</script>\n```", &opts());
        assert_eq!(doc.blocks.len(), 2, "{doc:?}");
        let Block::Code { lang, text } = &doc.blocks[1] else { panic!() };
        assert_eq!(lang.as_deref(), Some("html"));
        assert_eq!(text.as_str(), "<script>x</script>");
    }

    #[test]
    fn nested_lists_and_tasks() {
        let doc = parse("- a\n  - b\n  - c\n- [x] done\n- [ ] todo\n\n1. one\n2. two", &opts());
        let Block::List { ordered: false, items, .. } = &doc.blocks[0] else { panic!("{doc:?}") };
        assert_eq!(items.len(), 3);
        assert!(items[0].blocks.iter().any(|b| matches!(b, Block::List { items, .. } if items.len() == 2)));
        assert_eq!(items[1].checked, Some(true));
        assert_eq!(items[2].checked, Some(false));
        assert!(matches!(doc.blocks[1], Block::List { ordered: true, .. }));
    }

    #[test]
    fn table() {
        let doc = parse("| a | b |\n|---|:-:|\n| 1 | 2 |\n", &opts());
        let Block::Table { rows, weights } = &doc.blocks[0] else { panic!("{doc:?}") };
        assert_eq!(rows.len(), 2);
        assert!(rows[0][0].header);
        assert_eq!(rows[0][1].align, Align::Center);
        assert_eq!(weights.len(), 2);
    }

    #[test]
    fn details_summary() {
        let doc = parse("<details>\n<summary>More</summary>\n\nhidden **text**\n\n</details>", &opts());
        let Block::Details { summary, blocks, open } = &doc.blocks[0] else { panic!("{doc:?}") };
        assert_eq!(text_of(summary), "More");
        assert!(!open);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn breaks_option() {
        let mut o = opts();
        o.breaks = true;
        let doc = parse("line1\nline2", &o);
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!() };
        assert_eq!(text_of(flows), "line1\nline2");
        let doc = parse("line1\nline2", &opts());
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!() };
        assert_eq!(text_of(flows), "line1 line2");
    }

    #[test]
    fn picture_prefers_dark_source() {
        let doc = parse(
            "<picture><source media=\"(prefers-color-scheme: dark)\" srcset=\"dark.png\"><img src=\"light.png\"></picture>",
            &opts(),
        );
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!("{doc:?}") };
        assert_eq!(text_of(flows), "[img https://raw.githubusercontent.com/u/r/main/dark.png]");
    }

    fn check_cover(blocks: &[Block]) {
        let check_flows = |flows: &[Flow]| {
            for f in flows {
                if let Flow::Text(t) = f {
                    let mut at = 0;
                    for s in &t.spans {
                        assert_eq!(s.range.start, at, "gap in {:?}", t.text);
                        assert!(t.text.is_char_boundary(s.range.start) && t.text.is_char_boundary(s.range.end));
                        at = s.range.end;
                    }
                    assert_eq!(at, t.text.len(), "short cover {:?}", t.text);
                }
            }
        };
        for b in blocks {
            match b {
                Block::Heading { flows, .. } | Block::Paragraph { flows, .. } => check_flows(flows),
                Block::Quote(b) => check_cover(b),
                Block::List { items, .. } => items.iter().for_each(|i| check_cover(&i.blocks)),
                Block::Table { rows, .. } => rows.iter().flatten().for_each(|c| check_cover(&c.blocks)),
                Block::Details { summary, blocks, .. } => {
                    check_flows(summary);
                    check_cover(blocks);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn spans_always_cover_text() {
        let src = "a  *b*  \nc\n\n<p>x <b>y</b> \u{1F600} <br> z<br>\n<br>w</p>\n\n- `x` **y** \u{e9}\n  - [l](http://a)  \n\n## \u{df}ig `c` *d*\n\n| a | `b` |\n|--|--|\n| **1** | 2  3 |\n\n<details><summary> S </summary>  t </details>\n\n<img src=a.png> t <img src=b.png>";
        for breaks in [false, true] {
            for upper in [false, true] {
                let mut o = opts();
                o.breaks = breaks;
                o.uppercase_headings = upper;
                check_cover(&parse(src, &o).blocks);
            }
        }
    }

    #[test]
    fn dangerous_link_is_dropped() {
        let doc = parse("[x](javascript:alert(1))", &opts());
        let Block::Paragraph { flows, .. } = &doc.blocks[0] else { panic!() };
        let Flow::Text(t) = &flows[0] else { panic!() };
        assert!(t.spans.iter().all(|s| s.link.is_none()));
    }

    fn doc_depth(blocks: &[Block]) -> usize {
        blocks
            .iter()
            .map(|b| match b {
                Block::Quote(b) | Block::Details { blocks: b, .. } => 1 + doc_depth(b),
                Block::List { items, .. } => 1 + items.iter().map(|i| doc_depth(&i.blocks)).max().unwrap_or(0),
                Block::Table { rows, .. } => 1 + rows.iter().flatten().map(|c| doc_depth(&c.blocks)).max().unwrap_or(0),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    fn all_text(blocks: &[Block], out: &mut String) {
        for b in blocks {
            match b {
                Block::Heading { flows, .. } | Block::Paragraph { flows, .. } => out.push_str(&text_of(flows)),
                Block::Code { text, .. } => out.push_str(text),
                Block::Quote(b) | Block::Details { blocks: b, .. } => all_text(b, out),
                Block::List { items, .. } => items.iter().for_each(|i| all_text(&i.blocks, out)),
                Block::Table { rows, .. } => rows.iter().flatten().for_each(|c| all_text(&c.blocks, out)),
                Block::Rule => {}
            }
        }
    }

    #[test]
    fn deep_nesting_is_capped_and_keeps_text() {
        let n = 5000;
        let sources = [
            ">".repeat(n) + " deep",
            "- ".repeat(n) + "deep",
            "<div>".repeat(n) + "deep",
            "<b>".repeat(n) + "deep",
            "<blink>".repeat(n) + "deep",
            "<details open>".repeat(n) + "deep",
            "<table><tr><td>".repeat(n) + "deep",
        ];
        for src in sources {
            let head: String = src.chars().take(16).collect();
            // 1 MiB: below the stack of every thread that parses (uncapped, debug builds overflowed it at ~500 levels).
            let doc = std::thread::Builder::new()
                .stack_size(1 << 20)
                .spawn(move || parse(&src, &opts()))
                .unwrap()
                .join()
                .unwrap();
            assert!(doc_depth(&doc.blocks) <= super::super::markdown_html::MAX_DEPTH, "{head}");
            let mut text = String::new();
            all_text(&doc.blocks, &mut text);
            assert!(text.contains("deep"), "{head}: {text:?}");
        }
        // Nesting below the cap is untouched.
        let doc = parse(&(">".repeat(20) + " x"), &opts());
        assert_eq!(doc_depth(&doc.blocks), 20);
    }
}
