//! Shared state of the dashboard page (selected addon, search text), plus the pure list
//! logic: filtering, sorting and keeping the selection valid.
//!
//! The model lives in a gpui `Global`, so the selected addon survives page switches. The page constructor resets the
//! search query because the toolbar's search box always starts empty.
use std::{cmp::Ordering, collections::HashMap};

use gpui_kit::{App, AppContext as _, Context, Entity, Global};

use crate::{
    backend::{
        addon::{self, Addon},
        api::Release,
    },
    state::catalog::locale_cmp,
    ui::utils::AddonSearch,
};

#[derive(Default)]
pub struct DashboardModel {
    /// The selected addon as a snapshot. Resolve it against the store with [`resolve_selected`].
    pub selected: Option<Addon>,
    /// Text typed into the toolbar search box.
    pub search_query: String,
}

struct ModelGlobal(Entity<DashboardModel>);
impl Global for ModelGlobal {}

/// The shared model (created on first use).
pub fn model(cx: &mut App) -> Entity<DashboardModel> {
    if let Some(g) = cx.try_global::<ModelGlobal>() {
        return g.0.clone();
    }
    let entity = cx.new(|_| DashboardModel::default());
    cx.set_global(ModelGlobal(entity.clone()));
    entity
}

impl DashboardModel {
    pub fn set_selected(&mut self, addon: Option<Addon>, cx: &mut Context<Self>) {
        self.selected = addon;
        cx.notify();
    }
}

/// Keeps the addons matching the search text in name, alias, description or author, then sorts them: addons with an
/// update first, then by name.
pub fn filtered_addons(installed: &[Addon], releases: &HashMap<String, Release>, search_query: &str) -> Vec<Addon> {
    let search = AddonSearch::new(search_query);
    let mut filtered: Vec<Addon> =
        installed.iter().filter(|a| search.matches(&a.name, &a.alias, &a.description, &a.author)).cloned().collect();
    filtered.sort_by(|a, b| {
        let a_up = releases.get(&a.name).is_some_and(|r| addon::has_update(a, r));
        let b_up = releases.get(&b.name).is_some_and(|r| addon::has_update(b, r));
        match (a_up, b_up) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => locale_cmp(&a.name, &b.name),
        }
    });
    filtered
}

/// The stored snapshot, replaced by the live addon with the same name when that addon is installed.
pub fn resolve_selected(installed: &[Addon], snapshot: Option<&Addon>) -> Option<Addon> {
    snapshot.map(|s| installed.iter().find(|a| a.name == s.name).unwrap_or(s).clone())
}

/// Keeps the selection valid for the current list. Returns `Some(new_selection)` when the selection has to change:
/// with nothing selected the first listed addon is picked, and when the selected addon is no longer listed the first
/// one (or nothing) replaces it.
pub fn reconcile_selection(selected: Option<&Addon>, filtered: &[Addon]) -> Option<Option<Addon>> {
    match selected {
        None => filtered.first().map(|first| Some(first.clone())),
        Some(sel) => {
            if filtered.iter().any(|a| a.name == sel.name) {
                None
            } else {
                Some(filtered.first().cloned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn addon(name: &str, alias: &str, managed: bool, version: &str) -> Addon {
        Addon {
            name: name.into(),
            alias: alias.into(),
            description: format!("desc of {name}"),
            version: version.into(),
            is_managed: managed,
            updated_at: Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            ..Addon::default()
        }
    }

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            published_at: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
            ..Release::default()
        }
    }

    #[test]
    fn sorts_updates_first_then_by_name() {
        let installed =
            vec![addon("zeta", "Zeta", true, "1"), addon("alpha", "Alpha", true, "1"), addon("mid", "Mid", true, "1")];
        let mut releases = HashMap::new();
        releases.insert("zeta".to_string(), release("2"));
        releases.insert("alpha".to_string(), release("1"));
        let out = filtered_addons(&installed, &releases, "");
        let names: Vec<_> = out.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn unmanaged_never_counts_as_update() {
        let installed = vec![addon("b", "B", false, ""), addon("a", "A", true, "1")];
        let mut releases = HashMap::new();
        releases.insert("b".to_string(), release("9"));
        let names: Vec<_> = filtered_addons(&installed, &releases, "").into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn search_matches_name_alias_description_author() {
        let mut installed = vec![addon("dbm", "Deadly Boss Mods", true, "1"), addon("bag", "Bagnon", true, "1")];
        installed[1].author = "Tuller".into();
        let r = HashMap::new();
        assert_eq!(filtered_addons(&installed, &r, "DEADLY").len(), 1);
        assert_eq!(filtered_addons(&installed, &r, "bag")[0].name, "bag");
        assert_eq!(filtered_addons(&installed, &r, "desc of").len(), 2);
        assert_eq!(filtered_addons(&installed, &r, "tuller")[0].name, "bag");
        assert_eq!(filtered_addons(&installed, &r, "   ").len(), 2);
        assert!(filtered_addons(&installed, &r, "nothing").is_empty());
    }

    #[test]
    fn selection_reconcile() {
        let a = addon("a", "A", true, "1");
        let b = addon("b", "B", true, "1");
        let list = vec![a.clone(), b.clone()];
        assert_eq!(reconcile_selection(None, &list), Some(Some(a.clone())));
        assert_eq!(reconcile_selection(None, &[]), None);
        assert_eq!(reconcile_selection(Some(&b), &list), None);
        assert_eq!(reconcile_selection(Some(&addon("gone", "G", true, "1")), &list), Some(Some(a)));
        assert_eq!(reconcile_selection(Some(&b), &[]), Some(None));
    }

    #[test]
    fn resolve_prefers_live_addon() {
        let old = addon("a", "Old", true, "1");
        let live = addon("a", "New", true, "2");
        assert_eq!(resolve_selected(std::slice::from_ref(&live), Some(&old)).unwrap().version, "2");
        assert_eq!(resolve_selected(&[], Some(&old)).unwrap().alias, "Old");
        assert!(resolve_selected(&[live], None).is_none());
    }
}
