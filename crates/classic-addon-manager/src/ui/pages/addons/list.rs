//! The addon list of the catalog page: skeletons while loading, error and empty states, and the "New" and "All other
//! addons" sections for the list and grid layouts (one flat section when a sort order is picked).
use std::rc::Rc;

use cam_core::config::{AddonSort, AddonViewMode};
use cam_ui::{
    anim::{VIEW_FADE_DURATION, fade_in},
    components::{Button, ButtonSize, ButtonVariant, EmptyState, EmptyTone, Skeleton},
    icons,
    theme::{Tokens, TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

use super::{
    catalog::{AddonListItem, CatalogState},
    item::{COL_ACTION, COL_DOWNLOADS, COL_GAP, COL_ICON, COL_LIKED, COL_TAGS, OpenHandler, RemoteAddon, list_header},
};
use crate::ui::{
    addon_filter::{NEW_ADDON_DAYS, is_new_addon},
    format::truncate_chars,
};

pub type RetryHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Narrowest a grid card may get: the stats footer needs this much room next to "Installed".
pub const GRID_MIN_COL: f32 = 220.0;
pub const GRID_GAP: f32 = 12.0;

/// Number of grid columns that fit in `width` pixels, each at least [`GRID_MIN_COL`] wide with [`GRID_GAP`] between
/// them. Always at least one.
pub fn grid_columns(width: f32) -> usize {
    (((width + GRID_GAP) / (GRID_MIN_COL + GRID_GAP)).floor() as usize).max(1)
}

/// A rounded, bordered group with 1px dividers between the rows.
fn list_group(rows: Vec<AnyElement>, t: &Tokens) -> impl IntoElement {
    let border = alpha(t.border, 0.6);
    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(radius::XL)
        .border_1()
        .border_color(border)
        .bg(alpha(t.muted, 0.2))
        .children(
            rows.into_iter()
                .enumerate()
                .map(move |(i, row)| div().min_w_0().when(i > 0, |d| d.border_t_1().border_color(border)).child(row)),
        )
}

fn grid_group(cells: Vec<AnyElement>, cols: usize) -> impl IntoElement {
    div().grid().grid_cols(cols as u16).gap(px(GRID_GAP)).children(cells.into_iter().map(|c| div().min_w_0().child(c)))
}

/// Tiny uppercase caption with a trailing hairline. `highlight` adds a dot in the primary colour.
fn section_label(
    text: impl Into<SharedString>,
    highlight: bool,
    mx: f32,
    mb: f32,
    mt: f32,
    t: &Tokens,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .mx(px(mx))
        .mb(px(mb))
        .mt(px(mt))
        .text_size(px(11.0))
        .line_height(px(16.5))
        .text_color(t.muted_foreground)
        .when(highlight, |d| d.child(div().flex_none().size(px(6.0)).rounded_full().bg(t.primary)))
        .child(text.into().to_uppercase())
        .child(div().h(px(1.0)).flex_1().bg(alpha(t.border, 0.6)))
}

// ---------------------------------------------------------------------------------------------------------
// skeleton
// ---------------------------------------------------------------------------------------------------------

/// Placeholder card shown in the grid view while the catalog loads.
fn skeleton_card(t: &Tokens) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .rounded(radius::XL)
        .border_1()
        .border_color(alpha(t.border, 0.6))
        .bg(alpha(t.muted, 0.2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(12.0))
                .pt(px(12.0))
                .child(Skeleton::new().size(px(40.0)).flex_none().rounded(radius::LG))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(6.0))
                        .child(Skeleton::new().h(px(14.0)).w(relative(0.75)))
                        .child(Skeleton::new().h(px(12.0)).w(relative(0.5))),
                ),
        )
        .child(
            div()
                .mt(px(10.0))
                .flex()
                .flex_col()
                .justify_center()
                .min_h(px(32.0))
                .gap(px(6.0))
                .px(px(12.0))
                .child(Skeleton::new().h(px(10.0)).w_full())
                .child(Skeleton::new().h(px(10.0)).w(relative(0.6667))),
        )
        .child(
            div()
                .mb(px(10.0))
                .mt(px(8.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(20.0))
                .px(px(12.0))
                .child(Skeleton::new().h(px(16.0)).w(px(48.0)).rounded_full())
                .child(Skeleton::new().h(px(16.0)).w(px(48.0)).rounded_full()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .h(px(40.0))
                .px(px(12.0))
                .border_t_1()
                .border_color(alpha(t.border, 0.6))
                .child(Skeleton::new().h(px(12.0)).w(px(48.0)))
                .child(Skeleton::new().h(px(12.0)).w(px(36.0))),
        )
}

/// Placeholder row shown in the list view while the catalog loads.
fn skeleton_row() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(COL_GAP))
        .w_full()
        .px(px(12.0))
        .py(px(10.0))
        .child(Skeleton::new().size(px(COL_ICON)).flex_none().rounded(radius::LG))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(6.0))
                .child(Skeleton::new().h(px(16.0)).w(px(160.0)))
                .child(Skeleton::new().h(px(12.0)).w(px(256.0)).max_w_full()),
        )
        .child(div().flex_none().w(px(COL_TAGS)).child(Skeleton::new().h(px(12.0)).w(px(112.0))))
        .child(
            div().flex().flex_none().justify_end().w(px(COL_DOWNLOADS)).child(Skeleton::new().h(px(12.0)).w(px(48.0))),
        )
        .child(div().flex().flex_none().justify_end().w(px(COL_LIKED)).child(Skeleton::new().h(px(12.0)).w(px(36.0))))
        .child(div().flex_none().w(px(COL_ACTION)))
}

// ---------------------------------------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------------------------------------

fn retry_button(
    id: &'static str,
    variant: ButtonVariant,
    on_retry: &RetryHandler,
    class: fn(Button) -> Button,
) -> Button {
    let on_retry = on_retry.clone();
    class(
        Button::new(id)
            .variant(variant)
            .size(ButtonSize::Sm)
            .label("Retry")
            .on_click(move |_, window, cx| on_retry(window, cx)),
    )
}

/// "Couldn't refresh" banner shown above a stale list.
fn banner_el(err: &str, on_retry: &RetryHandler, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(radius::LG)
        .border_1()
        .border_color(t.border)
        .bg(t.card)
        .px(px(12.0))
        .py(px(8.0))
        .text_sm_tw()
        .text_color(t.muted_foreground)
        .child(Icon::new(icons::TRIANGLE_ALERT).size(px(16.0)).flex_none().text_color(tw(Tw::Orange, 500)))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(format!("Couldn't refresh addons: {}. Showing the last loaded list.", truncate_chars(err, 120))),
        )
        .child(retry_button("addons-banner-retry", ButtonVariant::Ghost, on_retry, |b| b))
        .into_any_element()
}

/// Everything the addon list shows except the populated list: skeletons, the load error and the "no addons" state.
/// `None` when there are rows to show (use [`Snapshot`] / [`rows`] / [`render_row`]). `filtered_empty`: no addon
/// matches the current search / tag.
pub fn static_view(
    state: &CatalogState,
    filtered_empty: bool,
    view_mode: AddonViewMode,
    sort: AddonSort,
    cols: usize,
    on_retry: &RetryHandler,
    t: &Tokens,
) -> Option<AnyElement> {
    if !state.is_ready {
        let skeletons = (0..7).map(|_| match view_mode {
            AddonViewMode::Grid => skeleton_card(t).into_any_element(),
            AddonViewMode::List => skeleton_row().into_any_element(),
        });
        return Some(match view_mode {
            AddonViewMode::Grid => {
                div().flex().flex_col().py(px(16.0)).child(grid_group(skeletons.collect(), cols)).into_any_element()
            }
            AddonViewMode::List => div()
                .flex()
                .flex_col()
                .py(px(16.0))
                .child(list_header(sort, t))
                .child(list_group(skeletons.collect(), t))
                .into_any_element(),
        });
    }

    let banner = match (&state.error, state.addons.is_empty()) {
        (Some(err), false) => Some(div().mt(px(16.0)).child(banner_el(err, on_retry, t))),
        _ => None,
    };

    let (key, body) = match &state.error {
        Some(err) if state.addons.is_empty() => (
            "addons-error",
            EmptyState::new(icons::TRIANGLE_ALERT, "Couldn't load addons")
                .tone(EmptyTone::Destructive)
                .description(truncate_chars(err, 200).to_string())
                .child(retry_button("addons-error-retry", ButtonVariant::Outline, on_retry, |b| b)),
        ),
        _ if filtered_empty => (
            "no-addons",
            EmptyState::new(icons::PACKAGE_SEARCH, "No addons found").description(if state.search_query.is_empty() {
                "No addons are currently available.\nCheck back later or try refreshing."
            } else {
                "No addons match your search criteria.\nTry adjusting your search or filters."
            }),
        ),
        _ => return None,
    };
    let body =
        fade_in(body.py(px(64.0)).text_color(t.muted_foreground), ElementId::Name(key.into()), VIEW_FADE_DURATION);

    Some(div().flex().flex_col().children(banner).child(body).into_any_element())
}

// ---------------------------------------------------------------------------------------------------------
// populated list (virtualized: one `Row` per `gpui_kit::list` item, elements are built only for visible rows)
// ---------------------------------------------------------------------------------------------------------

/// The filtered addons in display order (new first, then the rest, or flat in sort order) plus what the rows need
/// besides the addons.
pub struct Snapshot {
    items: Vec<AddonListItem>,
    /// `items[..new_count]` are the "New in the last N days" addons (always 0 for a non-default sort).
    new_count: usize,
    sort: AddonSort,
    /// Load error shown above a stale list.
    banner: Option<String>,
    max_downloads: i64,
}

impl Snapshot {
    /// `indices`: result of [`CatalogState::filtered_indices`] for `sort`.
    pub fn build(state: &CatalogState, indices: &[usize], sort: AddonSort) -> Self {
        let addons = indices.iter().map(|&i| &state.addons[i]);
        let (items, new_count) = if sort == AddonSort::Default {
            let (new, other): (Vec<&AddonListItem>, Vec<&AddonListItem>) =
                addons.partition(|a| is_new_addon(a.manifest.added_at));
            let new_count = new.len();
            (new.into_iter().chain(other).cloned().collect(), new_count)
        } else {
            (addons.cloned().collect(), 0)
        };
        Self {
            items,
            new_count,
            sort,
            banner: state.error.clone().filter(|_| !state.addons.is_empty()),
            max_downloads: state.max_downloads,
        }
    }
}

/// Caption above a group of addons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    /// "New in the last N days" (default order).
    New,
    /// "All other addons" (default order).
    Other,
    /// The whole filtered catalog in the picked sort order.
    Sorted,
}

/// Caption of the flat sorted section, e.g. "All addons · most downloaded first".
fn sorted_caption(sort: AddonSort) -> &'static str {
    match sort {
        AddonSort::Default => "All addons",
        AddonSort::NameAsc => "All addons · A to Z",
        AddonSort::NameDesc => "All addons · Z to A",
        AddonSort::DownloadsDesc => "All addons · most downloaded first",
        AddonSort::DownloadsAsc => "All addons · least downloaded first",
        AddonSort::RatingDesc => "All addons · highest rated first",
        AddonSort::RatingAsc => "All addons · lowest rated first",
    }
}

/// One item of the virtualized list. Compared to detect layout changes (then the list state is reset).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Spacer,
    Banner,
    ListHeader,
    /// Section caption.
    Label(Section),
    /// List mode: `items[index]`; `first` / `last` mark the ends of its section (rounded, bordered group).
    ListItem {
        index: usize,
        first: bool,
        last: bool,
    },
    /// Grid mode: `items[start..end]` laid out on the `cols` column grid.
    Grid {
        start: usize,
        end: usize,
        last: bool,
    },
}

/// Rows for `snapshot` in display order. `cols` is the grid column count.
pub fn rows(snapshot: &Snapshot, view_mode: AddonViewMode, cols: usize) -> Vec<Row> {
    let (len, new) = (snapshot.items.len(), snapshot.new_count);
    let sorted = snapshot.sort != AddonSort::Default;
    let mut rows = Vec::new();
    if snapshot.banner.is_some() {
        rows.push(Row::Banner);
    }
    rows.push(Row::Spacer);
    if view_mode == AddonViewMode::List {
        rows.push(Row::ListHeader);
    }
    for (range, section) in [(0..new, Section::New), (new..len, if sorted { Section::Sorted } else { Section::Other })]
    {
        if range.is_empty() {
            continue;
        }
        if new > 0 || sorted {
            rows.push(Row::Label(section));
        }
        match view_mode {
            AddonViewMode::List => rows.extend(range.clone().map(|index| Row::ListItem {
                index,
                first: index == range.start,
                last: index + 1 == range.end,
            })),
            AddonViewMode::Grid => rows.extend(range.clone().step_by(cols.max(1)).map(|start| {
                let end = (start + cols.max(1)).min(range.end);
                Row::Grid { start, end, last: end == range.end }
            })),
        }
    }
    rows.push(Row::Spacer);
    rows
}

/// Builds the element of one row. Called by `gpui_kit::list` for visible rows only.
pub fn render_row(
    row: Row,
    snapshot: &Snapshot,
    view_mode: AddonViewMode,
    cols: usize,
    on_open: &OpenHandler,
    on_retry: &RetryHandler,
    cx: &mut App,
) -> AnyElement {
    let t = cx.tokens();
    let cell = |i: usize| {
        let a = &snapshot.items[i];
        RemoteAddon::new(a.manifest.clone(), a.is_installed, snapshot.max_downloads, view_mode, on_open.clone())
            .into_any_element()
    };
    let row_el = div().w_full();
    match row {
        Row::Spacer => row_el.h(px(16.0)),
        Row::Banner => row_el.pt(px(16.0)).children(snapshot.banner.as_deref().map(|err| banner_el(err, on_retry, &t))),
        Row::ListHeader => row_el.child(list_header(snapshot.sort, &t)),
        Row::Label(section) => {
            let (mx, mb, mt) = match (view_mode, section) {
                (AddonViewMode::Grid, Section::Other) => (0.0, 10.0, 20.0),
                (AddonViewMode::Grid, _) => (0.0, 10.0, 0.0),
                (AddonViewMode::List, _) => (12.0, 6.0, 12.0),
            };
            let text = match section {
                Section::New => format!("New in the last {NEW_ADDON_DAYS} days"),
                Section::Other => "All other addons".to_string(),
                Section::Sorted => sorted_caption(snapshot.sort).to_string(),
            };
            row_el.pt(px(mt)).pb(px(mb)).child(section_label(text, section == Section::New, mx, 0.0, 0.0, &t))
        }
        Row::ListItem { index, first, last } => {
            // The bordered group of `list_group` split per row: bordered, rounded at the section's ends, 1px dividers
            // between rows.
            let border = alpha(t.border, 0.6);
            row_el
                .min_w_0()
                .overflow_hidden()
                .bg(alpha(t.muted, 0.2))
                .border_l_1()
                .border_r_1()
                .border_color(border)
                .when(first, |d| d.border_t_1().rounded_t(radius::XL))
                .when(last, |d| d.border_b_1().rounded_b(radius::XL))
                .child(div().min_w_0().when(!first, |d| d.border_t_1().border_color(border)).child(cell(index)))
        }
        Row::Grid { start, end, last } => row_el.when(!last, |d| d.pb(px(GRID_GAP))).child(
            div()
                .grid()
                .grid_cols(cols as u16)
                .gap(px(GRID_GAP))
                .children((start..end).map(|i| div().min_w_0().child(cell(i)))),
        ),
    }
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(total: usize, new_count: usize, banner: bool) -> Snapshot {
        Snapshot {
            items: (0..total).map(|_| AddonListItem { manifest: Default::default(), is_installed: false }).collect(),
            new_count,
            sort: AddonSort::default(),
            banner: banner.then(|| "boom".to_string()),
            max_downloads: 0,
        }
    }

    #[test]
    fn grid_rows_chunk_each_section_by_columns() {
        let r = rows(&snapshot(7, 3, false), AddonViewMode::Grid, 2);
        assert_eq!(
            r,
            vec![
                Row::Spacer,
                Row::Label(Section::New),
                Row::Grid { start: 0, end: 2, last: false },
                Row::Grid { start: 2, end: 3, last: true },
                Row::Label(Section::Other),
                Row::Grid { start: 3, end: 5, last: false },
                Row::Grid { start: 5, end: 7, last: true },
                Row::Spacer,
            ]
        );
    }

    #[test]
    fn list_rows_mark_section_ends_and_skip_labels_without_new_addons() {
        let r = rows(&snapshot(3, 0, true), AddonViewMode::List, 4);
        assert_eq!(
            r,
            vec![
                Row::Banner,
                Row::Spacer,
                Row::ListHeader,
                Row::ListItem { index: 0, first: true, last: false },
                Row::ListItem { index: 1, first: false, last: false },
                Row::ListItem { index: 2, first: false, last: true },
                Row::Spacer,
            ]
        );
    }

    #[test]
    fn sorted_rows_are_one_labelled_section() {
        let mut s = snapshot(3, 0, false);
        s.sort = AddonSort::DownloadsDesc;
        assert_eq!(
            rows(&s, AddonViewMode::Grid, 2),
            vec![
                Row::Spacer,
                Row::Label(Section::Sorted),
                Row::Grid { start: 0, end: 2, last: false },
                Row::Grid { start: 2, end: 3, last: true },
                Row::Spacer,
            ]
        );
    }
}
