//! HTML fragment parsing and sanitising for the markdown widget.
//!
//! `sanitize_markdown_html(html)` parses `html` like a browser would (html5ever) and drops everything that is not on
//! an allow-list (scripts, styles, iframes, forms, svg, event handlers, `style=`, `javascript:` urls, comments ...).
//! Elements that are neither allowed nor dropped are unwrapped: the element goes away but its text and allowed
//! children stay. The result is serialised again.
//! The widget itself uses [`parse_fragment`] + [`sanitize_dom`] directly and never re-serialises.
extern crate markup5ever_rcdom as rcdom;

use std::rc::Rc;

use html5ever::serialize::{SerializeOpts, TraversalScope, serialize};
use html5ever::tendril::TendrilSink;
use html5ever::{LocalName, ParseOpts, QualName, local_name, ns};
use rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};

/// Elements that are kept (everything else is unwrapped or dropped).
const ALLOWED_TAGS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "article",
    "aside",
    "b",
    "bdi",
    "bdo",
    "big",
    "blockquote",
    "br",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "font",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "img",
    "input",
    "ins",
    "kbd",
    "li",
    "main",
    "mark",
    "nav",
    "ol",
    "p",
    "picture",
    "pre",
    "q",
    "s",
    "samp",
    "section",
    "small",
    "source",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "tt",
    "u",
    "ul",
    "var",
    "wbr",
];

/// Elements removed together with everything inside them (scripts, styles, embedded documents, form controls ...).
const DROP_WITH_CONTENT: &[&str] = &[
    "script",
    "style",
    "iframe",
    "frame",
    "frameset",
    "object",
    "embed",
    "applet",
    "noscript",
    "noembed",
    "noframes",
    "template",
    "title",
    "head",
    "svg",
    "math",
    "select",
    "textarea",
    "xmp",
    "plaintext",
    "audio",
    "video",
    "canvas",
    "link",
    "meta",
    "base",
    "button",
    "form",
    "dialog",
    "option",
    "optgroup",
];

/// Attributes that are kept on every allowed element.
const ALLOWED_ATTRS: &[&str] = &[
    "href", "src", "srcset", "alt", "title", "width", "height", "align", "valign", "colspan", "rowspan", "open",
    "checked", "disabled", "type", "start", "reversed", "media", "class", "lang", "dir", "id", "name", "cite",
];

/// Attributes whose value is a url and must pass [`is_safe_url`].
const URL_ATTRS: &[&str] = &["href", "src", "cite", "srcset"];

/// Element nesting kept by [`sanitize_dom`]. Deeper elements are unwrapped to their text, so everything downstream
/// that recurses over the tree (the sanitiser itself, the `Doc` builder, `readme.rs` rendering, gpui layout) has a
/// bounded depth. Without it a README of ~800 nested `>` overflows the main thread's stack and aborts the app.
/// Real documents (nested lists inside tables inside `<details>`, plus inline marks) stay well below this.
pub const MAX_DEPTH: usize = 32;

/// A parsed body fragment. Keeps the `RcDom` alive: dropping the document node empties every descendant's
/// children (rcdom's iterative `Drop`), so the root must not outlive it.
pub struct Fragment {
    _dom: RcDom,
    root: Handle,
}

impl Fragment {
    /// Synthetic root element whose children are the fragment's nodes.
    pub fn root(&self) -> &Handle {
        &self.root
    }
}

/// Parses `html` as a body fragment.
pub fn parse_fragment(html: &str) -> Fragment {
    let dom = html5ever::parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        QualName::new(None, ns!(html), local_name!("div")),
        vec![],
        true,
    )
    .one(html);
    let first = dom.document.children.borrow().first().cloned();
    let root = first.unwrap_or_else(|| dom.document.clone());
    Fragment { _dom: dom, root }
}

/// Sanitises `html` and returns the safe HTML.
pub fn sanitize_markdown_html(html: &str) -> String {
    let fragment = parse_fragment(html);
    sanitize_dom(fragment.root());
    let mut out = Vec::new();
    let opts = SerializeOpts { traversal_scope: TraversalScope::ChildrenOnly(None), ..Default::default() };
    if serialize(&mut out, &SerializableHandle::from(fragment.root().clone()), opts).is_err() {
        return String::new();
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Sanitises the children of `root` in place. Element nesting below `root` is capped at [`MAX_DEPTH`].
pub fn sanitize_dom(root: &Handle) {
    let old: Vec<Handle> = std::mem::take(&mut *root.children.borrow_mut());
    let mut new: Vec<Handle> = Vec::with_capacity(old.len());
    for child in old {
        sanitize_into(&child, root, 0, &mut new);
    }
    *root.children.borrow_mut() = new;
}

/// `depth`: number of elements between `node` and the root, counted on the input (unwrapped elements count too, as
/// they cost a stack frame here).
fn sanitize_into(node: &Handle, parent: &Handle, depth: usize, out: &mut Vec<Handle>) {
    match &node.data {
        NodeData::Text { .. } => {
            node.parent.set(Some(Rc::downgrade(parent)));
            out.push(node.clone());
        }
        NodeData::Element { name, attrs, .. } => {
            let tag: &str = &name.local;
            if DROP_WITH_CONTENT.contains(&tag) {
                return;
            }
            if depth >= MAX_DEPTH {
                flatten_text_into(node, parent, out);
                return;
            }
            if !ALLOWED_TAGS.contains(&tag) {
                // Unknown element: keep its (sanitised) children in its place.
                let kids: Vec<Handle> = std::mem::take(&mut *node.children.borrow_mut());
                for kid in kids {
                    sanitize_into(&kid, parent, depth + 1, out);
                }
                return;
            }
            let is_data_uri_tag = matches!(tag, "img" | "source");
            attrs.borrow_mut().retain(|attr| {
                let key: &str = &attr.name.local;
                if !attr.name.ns.is_empty() || key.starts_with("on") || !ALLOWED_ATTRS.contains(&key) {
                    return false;
                }
                if tag == "input" && key == "type" && !attr.value.eq_ignore_ascii_case("checkbox") {
                    return false;
                }
                if URL_ATTRS.contains(&key) {
                    return if key == "srcset" {
                        attr.value
                            .split(',')
                            .all(|c| is_safe_url(c.split_whitespace().next().unwrap_or(""), is_data_uri_tag))
                    } else {
                        is_safe_url(&attr.value, is_data_uri_tag && key == "src")
                    };
                }
                true
            });
            if tag == "input" {
                let is_checkbox = attrs.borrow().iter().any(|a| &*a.name.local == "type");
                if !is_checkbox {
                    return;
                }
            }
            let kids: Vec<Handle> = std::mem::take(&mut *node.children.borrow_mut());
            let mut cleaned = Vec::with_capacity(kids.len());
            for kid in kids {
                sanitize_into(&kid, node, depth + 1, &mut cleaned);
            }
            *node.children.borrow_mut() = cleaned;
            node.parent.set(Some(Rc::downgrade(parent)));
            out.push(node.clone());
        }
        // comments, doctype, processing instructions, nested documents
        _ => {}
    }
}

/// Replaces the element `node` by the text nodes of its subtree (minus [`DROP_WITH_CONTENT`] subtrees), in document
/// order. Iterative, so the subtree may be arbitrarily deep.
fn flatten_text_into(node: &Handle, parent: &Handle, out: &mut Vec<Handle>) {
    let mut stack = vec![node.clone()];
    while let Some(n) = stack.pop() {
        match &n.data {
            NodeData::Text { .. } => {
                n.parent.set(Some(Rc::downgrade(parent)));
                out.push(n);
            }
            NodeData::Element { name, .. } if !DROP_WITH_CONTENT.contains(&&*name.local) => {
                let kids: Vec<Handle> = std::mem::take(&mut *n.children.borrow_mut());
                stack.extend(kids.into_iter().rev());
            }
            _ => {}
        }
    }
}

/// Whether a url attribute value may be kept. Relative urls and the `http(s)`, `mailto`, `tel` ... schemes are fine.
/// `javascript:`, `vbscript:` and `data:` urls are not, except `data:image/*` in image sources (`allow_data_image`).
pub fn is_safe_url(value: &str, allow_data_image: bool) -> bool {
    // Whitespace and control characters are removed before the scheme is tested, because browsers ignore them when
    // opening a link (`jav\tascript:` still runs a script).
    let cleaned: String = value
        .chars()
        .filter(|c| {
            !(*c <= '\u{20}'
                || matches!(*c, '\u{a0}' | '\u{1680}' | '\u{180e}' | '\u{2000}'..='\u{2029}' | '\u{205f}' | '\u{3000}'))
        })
        .collect();
    let lower = cleaned.to_ascii_lowercase();
    // No scheme (relative url, `#anchor`, `/path`, `?query`): `:` only appears after the first `/ ? #`.
    let scheme_end = lower.find(':');
    let first_delim = lower.find(['/', '?', '#']);
    let Some(colon) = scheme_end else { return true };
    if first_delim.is_some_and(|d| d < colon) {
        return true;
    }
    let scheme = &lower[..colon];
    match scheme {
        "http" | "https" | "ftp" | "ftps" | "mailto" | "tel" | "callto" | "sms" | "cid" | "xmpp" | "matrix" => true,
        "data" => allow_data_image && lower["data:".len()..].starts_with("image/"),
        _ => false,
    }
}

/// Convenience for the AST builder: attribute lookup on an element handle.
pub fn attr(node: &Node, key: &str) -> Option<String> {
    if let NodeData::Element { attrs, .. } = &node.data {
        attrs.borrow().iter().find(|a| &*a.name.local == key).map(|a| a.value.to_string())
    } else {
        None
    }
}

/// Lower-case local name of an element.
pub fn tag_name(node: &Node) -> Option<LocalName> {
    match &node.data {
        NodeData::Element { name, .. } => Some(name.local.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(html: &str) -> String {
        sanitize_markdown_html(html)
    }

    #[test]
    fn drops_script_with_content() {
        assert_eq!(s("<p>a</p><script>alert(1)</script><p>b</p>"), "<p>a</p><p>b</p>");
        assert_eq!(s("x<SCRIPT src=//evil></SCRIPT>y"), "xy");
    }

    #[test]
    fn drops_style_iframe_object_svg() {
        assert_eq!(s("<style>p{color:red}</style>ok"), "ok");
        assert_eq!(s("<iframe src=\"https://e.com\"></iframe>ok"), "ok");
        assert_eq!(s("<object data=x></object><embed src=y>ok"), "ok");
        assert_eq!(s("<svg onload=alert(1)><circle/></svg>ok"), "ok");
        assert_eq!(s("<math><mi>x</mi></math>ok"), "ok");
    }

    #[test]
    fn strips_event_handlers_and_style_attr() {
        assert_eq!(s("<img src=\"a.png\" onerror=\"alert(1)\">"), "<img src=\"a.png\">");
        assert_eq!(s("<p onclick=\"x()\" style=\"color:red\" align=\"center\">t</p>"), "<p align=\"center\">t</p>");
        assert_eq!(s("<a href=\"https://x.y\" onmouseover=\"x()\">t</a>"), "<a href=\"https://x.y\">t</a>");
    }

    #[test]
    fn strips_dangerous_urls() {
        assert_eq!(s("<a href=\"javascript:alert(1)\">t</a>"), "<a>t</a>");
        assert_eq!(s("<a href=\"JaVaScRiPt:alert(1)\">t</a>"), "<a>t</a>");
        assert_eq!(s("<a href=\"jav&#x09;ascript:alert(1)\">t</a>"), "<a>t</a>");
        assert_eq!(s("<a href=\"  javascript:alert(1)\">t</a>"), "<a>t</a>");
        assert_eq!(s("<a href=\"vbscript:x\">t</a>"), "<a>t</a>");
        assert_eq!(s("<a href=\"data:text/html;base64,AAAA\">t</a>"), "<a>t</a>");
        assert_eq!(s("<img src=\"javascript:alert(1)\">"), "<img>");
    }

    #[test]
    fn keeps_safe_urls() {
        for url in [
            "https://a.b/c?d=e:f",
            "http://x",
            "mailto:a@b.c",
            "#anchor",
            "docs/img.png",
            "/abs/path",
            "./x:y",
            "?q=a:b",
        ] {
            let out = s(&format!("<a href=\"{url}\">t</a>"));
            assert!(out.contains("href"), "{url} => {out}");
        }
        assert_eq!(s("<img src=\"data:image/png;base64,AAAA\">"), "<img src=\"data:image/png;base64,AAAA\">");
        assert_eq!(s("<a href=\"data:image/png;base64,AAAA\">t</a>"), "<a>t</a>");
    }

    #[test]
    fn unwraps_unknown_and_form_elements() {
        assert_eq!(s("<blink>hi</blink>"), "hi");
        assert_eq!(s("<form action=\"x\"><input type=\"text\"><button>go</button></form>after"), "after");
        assert_eq!(s("<custom-el><b>bold</b></custom-el>"), "<b>bold</b>");
    }

    #[test]
    fn keeps_checkbox_inputs_only() {
        assert_eq!(
            s("<input type=\"checkbox\" checked disabled> x"),
            "<input type=\"checkbox\" checked=\"\" disabled=\"\"> x"
        );
        assert_eq!(s("<input type=\"text\" value=\"x\">"), "");
        assert_eq!(s("<input>"), "");
    }

    #[test]
    fn removes_comments() {
        assert_eq!(s("a<!-- <script>alert(1)</script> -->b"), "ab");
    }

    #[test]
    fn keeps_readme_html() {
        let html = "<p align=\"center\"><img src=\"logo.png\" width=\"200\" alt=\"Logo\"></p><details><summary>More</summary><p>x</p></details>";
        let out = s(html);
        assert!(out.contains("<p align=\"center\">"), "{out}");
        assert!(out.contains("<img src=\"logo.png\" width=\"200\" alt=\"Logo\">"), "{out}");
        assert!(out.contains("<details><summary>More</summary>"), "{out}");
    }

    #[test]
    fn srcset_is_checked() {
        assert_eq!(
            s("<picture><source srcset=\"a.png 1x, b.png 2x\"><img src=\"c.png\"></picture>"),
            "<picture><source srcset=\"a.png 1x, b.png 2x\"><img src=\"c.png\"></picture>"
        );
        assert_eq!(s("<source srcset=\"javascript:alert(1) 1x\">"), "<source>");
    }

    #[test]
    fn is_safe_url_cases() {
        assert!(is_safe_url("https://github.com/x", false));
        assert!(is_safe_url("relative/path:with-colon", false));
        assert!(!is_safe_url("javascript:alert(1)", false));
        assert!(!is_safe_url("\u{1}javascript:alert(1)", false));
        assert!(!is_safe_url("data:image/png;base64,AA", false));
        assert!(is_safe_url("data:image/png;base64,AA", true));
        assert!(!is_safe_url("data:text/html,<script>", true));
    }
}
