//! Callbacks the detail panes use to talk to the page (selection, list refresh, dropping a submission).
use gpui_kit::{App, Entity, Task, WeakEntity};

use super::{DeveloperPage, Selection, queries::DevQueries};

#[derive(Clone)]
pub struct Host {
    pub page: WeakEntity<DeveloperPage>,
    pub queries: Entity<DevQueries>,
}

impl Host {
    /// Selects this list entry.
    pub fn select(&self, selection: Selection, cx: &mut App) {
        let _ = self.page.update(cx, |page, cx| page.set_selection(Some(selection), cx));
    }

    /// Fetches everything the page showed and the list again (`DevQueries::retry`). Resolves when finished.
    pub fn refresh(&self, cx: &mut App) -> Task<()> {
        DevQueries::retry(&self.queries, cx)
    }

    /// Removes a submission from the cached list and clears the selection when it pointed at it.
    pub fn drop_submission(&self, id: i64, cx: &mut App) {
        self.queries.update(cx, |q, cx| q.remove_submission(id, cx));
        let _ = self.page.update(cx, |page, cx| {
            if page.selection == Some(Selection::Submission(id)) {
                page.set_selection(None, cx);
            }
        });
    }
}
