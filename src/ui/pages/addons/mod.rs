//! Addons catalog page: title bar toolbar, list or grid of the remote catalog, and the remote addon dialog.
mod catalog;
mod dialog;
mod item;
mod list;
mod stats;
mod toolbar;

use std::rc::Rc;

use gpui_kit::{
    AppContext as _, Context, ElementId, Entity, IntoElement, ListAlignment, ListState, ParentElement, Render, Styled,
    Window, div, list, px,
};

use self::{
    catalog::{CatalogState, catalog},
    dialog::RemoteAddonDialog,
    list::{RetryHandler, Row, Snapshot, grid_columns, render_row, rows as build_rows, static_view},
    toolbar::AddonsToolbar,
};
use crate::{
    backend::config::{AddonSort, AddonViewMode},
    state,
    ui::{
        anim::{VIEW_FADE_DURATION, fade_in},
        components::{ScrollArea, ScrollbarAxis, floating::window_area, scrollbar_overlay},
        theme::TokensExt,
        titlebar,
    },
};

/// Width of the app's sidebar, which sits next to the page.
const SIDEBAR_WIDTH: f32 = 220.0;

/// Widest the page content may get for a window of this width: the largest of 640, 768, 1024, 1280 and 1536 px that
/// fits in it. `None` for windows narrower than 640 px, where the content uses the full width.
pub fn container_max_width(viewport: f32) -> Option<f32> {
    [1536.0, 1280.0, 1024.0, 768.0, 640.0].into_iter().find(|bp| viewport >= *bp)
}

/// gpui's `list` drops every cached row height when its width changes but, with `measure_all`, only re-measures after
/// a `reset`. Re-arm the measuring (so the scrollbar keeps showing the full length) and stay where we were.
fn remeasure(list_state: &ListState, rows: usize) {
    let top = list_state.logical_scroll_top();
    list_state.reset(rows);
    list_state.scroll_to(top);
}

/// What the virtualized list currently shows, and the inputs it was built from.
struct ListView {
    rev: u64,
    indices: Vec<usize>,
    view_mode: AddonViewMode,
    sort: AddonSort,
    cols: usize,
    /// Window width the cached row heights were measured at.
    viewport: f32,
    snapshot: Rc<Snapshot>,
    rows: Rc<Vec<Row>>,
}

pub struct AddonsPage {
    catalog: Entity<CatalogState>,
    _toolbar: Entity<AddonsToolbar>,
    list_state: ListState,
    view: Option<ListView>,
}

impl AddonsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let catalog = catalog(cx);
        let toolbar = cx.new(|cx| AddonsToolbar::new(catalog.clone(), window, cx));
        titlebar::set_slot(cx, toolbar.clone());
        cx.observe(&catalog, |_, _, cx| cx.notify()).detach();
        cx.observe(&state::preferences(cx), |_, _, cx| cx.notify()).detach();
        // Fetch the catalog unless it is recent enough
        state::catalog::ensure(cx);
        // Measure every row on the first layout after a reset (cheap for ~100 rows) so the scrollbar thumb reflects the
        // full content height from the start instead of growing as rows come into view. Elements are still only
        // painted for visible rows; render a few hundred px beyond the viewport so scrolling does not pop in.
        let list_state = ListState::new(0, ListAlignment::Top, px(400.0)).measure_all();
        Self { catalog, _toolbar: toolbar, list_state, view: None }
    }
}

impl Render for AddonsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let prefs = state::preferences(cx).read(cx);
        let (view_mode, sort) = (prefs.addon_view_mode, prefs.addon_sort);

        // Content width: the page area (window minus sidebar) capped by `container_max_width`, minus 16 px of padding
        // on each side. The grid column count follows from it.
        let viewport: f32 = window_area(window).size.width.into();
        let page_width = (viewport - SIDEBAR_WIDTH).max(0.0);
        let container = container_max_width(viewport).map_or(page_width, |max| page_width.min(max));
        let cols = grid_columns((container - 32.0).max(0.0));

        let on_open: item::OpenHandler = Rc::new(RemoteAddonDialog::open);
        let on_retry: RetryHandler = Rc::new(|_, cx| state::catalog::refresh(cx).detach());

        let state = self.catalog.read(cx);
        let indices = state.filtered_indices(sort);

        // skeletons / error / "no addons": small, not worth virtualizing
        if let Some(content) = static_view(state, indices.is_empty(), view_mode, sort, cols, &on_retry, &t) {
            return div()
                .flex()
                .flex_col()
                .size_full()
                .min_h_0()
                .child(
                    ScrollArea::new("addons-scroll").flex_1().min_h_0().child(
                        div().flex().justify_center().w_full().child(
                            div()
                                .w_full()
                                .when_some(container_max_width(viewport), |d, max| d.max_w(px(max)))
                                .px(px(16.0))
                                .child(content),
                        ),
                    ),
                )
                .into_any_element();
        }

        // Rebuild the row model only when its inputs changed; reset the list (scroll to top, re-measure) only when
        // the rows themselves differ, so a background reload with the same result keeps the scroll position.
        let fresh = self.view.as_ref().is_some_and(|v| {
            v.rev == state.rev && v.view_mode == view_mode && v.sort == sort && v.cols == cols && v.indices == indices
        });
        let width_changed = self.view.as_ref().is_some_and(|v| v.viewport != viewport);
        if !fresh {
            let snapshot = Rc::new(Snapshot::build(state, &indices, sort));
            let rows = Rc::new(build_rows(&snapshot, view_mode, cols));
            // A new order starts from the top even when the layout is the same.
            let resorted = self.view.as_ref().is_some_and(|v| v.sort != sort);
            if resorted || self.view.as_ref().map(|v| &*v.rows) != Some(&*rows) {
                self.list_state.reset(rows.len());
            } else if width_changed {
                remeasure(&self.list_state, rows.len());
            }
            self.view = Some(ListView { rev: state.rev, indices, view_mode, sort, cols, viewport, snapshot, rows });
        } else if width_changed && let Some(v) = self.view.as_mut() {
            v.viewport = viewport;
            remeasure(&self.list_state, v.rows.len());
        }
        let view = self.view.as_ref().expect("list view was just built");
        let (snapshot, rows) = (view.snapshot.clone(), view.rows.clone());

        let rows_list = list(self.list_state.clone(), move |ix, _window, cx| {
            render_row(rows[ix], &snapshot, view_mode, cols, &on_open, &on_retry, cx)
        })
        .size_full();
        let key = ElementId::from(("addon-list", view_mode as usize));

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        div().flex().justify_center().size_full().child(
                            div()
                                .w_full()
                                .h_full()
                                .when_some(container_max_width(viewport), |d, max| d.max_w(px(max)))
                                .px(px(16.0))
                                .child(fade_in(div().size_full().child(rows_list), key, VIEW_FADE_DURATION)),
                        ),
                    )
                    .child(scrollbar_overlay("addons-scrollbar", &self.list_state, ScrollbarAxis::Vertical)),
            )
            .into_any_element()
    }
}

use gpui_kit::prelude::FluentBuilder as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_breakpoints() {
        assert_eq!(container_max_width(500.0), None);
        assert_eq!(container_max_width(640.0), Some(640.0));
        assert_eq!(container_max_width(985.0), Some(768.0));
        assert_eq!(container_max_width(1024.0), Some(1024.0));
        assert_eq!(container_max_width(1400.0), Some(1280.0));
        assert_eq!(container_max_width(2000.0), Some(1536.0));
    }

    #[test]
    fn grid_columns_follow_auto_fill() {
        // 985px window: 768px container, 32px padding => 736px => 3 columns of >= 220px with 12px gaps
        assert_eq!(grid_columns(736.0), 3);
        assert_eq!(grid_columns(220.0), 1);
        assert_eq!(grid_columns(451.0), 1);
        assert_eq!(grid_columns(452.0), 2);
        assert_eq!(grid_columns(0.0), 1);
    }
}
