//! Shared state of the dashboard page (selected addon, search text), plus the pure list
//! logic: filtering, sorting and keeping the selection valid.
//!
//! The model lives in a gpui `Global`, so the selected addon survives page switches. The page constructor resets the
//! search query because the toolbar's search box always starts empty.
use std::collections::HashMap;

use cam_core::{
    addon::{self, Addon},
    api::Release,
};
use gpui_kit::{App, AppContext as _, Context, Entity, Global};

use crate::{state::catalog::locale_cmp, ui::addon_filter::AddonSearch};

#[derive(Default)]
pub struct DashboardModel {
    /// The selected addon as a snapshot. Its name picks the addon: the page shows the installed addon with that name.
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

/// An addon of the installed list as the dashboard lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListedAddon {
    /// Position in the installed list.
    pub index: usize,
    pub has_update: bool,
}

/// Keeps the addons matching the search text in name, alias, description or author, then sorts them: addons with an
/// update first, then by name.
pub fn filtered_addons(
    installed: &[Addon],
    releases: &HashMap<String, Release>,
    search_query: &str,
) -> Vec<ListedAddon> {
    let search = AddonSearch::new(search_query);
    let mut listed: Vec<ListedAddon> = installed
        .iter()
        .enumerate()
        .filter(|(_, a)| search.matches(&a.name, &a.alias, &a.description, &a.author))
        .map(|(index, a)| ListedAddon {
            index,
            has_update: releases.get(&a.name).is_some_and(|r| addon::has_update(a, r)),
        })
        .collect();
    listed.sort_by(|a, b| {
        b.has_update.cmp(&a.has_update).then_with(|| locale_cmp(&installed[a.index].name, &installed[b.index].name))
    });
    listed
}

/// Keeps the selection valid for the listed addons. `selected` is the name of the selected addon. Returns
/// `Some(new_selection)` when the selection has to change: with nothing selected the first listed addon is picked, and
/// when the selected addon is no longer listed the first one (or nothing) replaces it.
pub fn reconcile_selection<'a>(
    selected: Option<&str>,
    listed: impl IntoIterator<Item = &'a Addon>,
) -> Option<Option<&'a Addon>> {
    let mut listed = listed.into_iter();
    let first = listed.next();
    match selected {
        None => first.map(Some),
        Some(name) => {
            let still_listed = first.is_some_and(|a| a.name == name) || listed.any(|a| a.name == name);
            (!still_listed).then_some(first)
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

    fn listed_names<'a>(installed: &'a [Addon], releases: &HashMap<String, Release>, query: &str) -> Vec<&'a str> {
        filtered_addons(installed, releases, query).iter().map(|l| installed[l.index].name.as_str()).collect()
    }

    #[test]
    fn sorts_updates_first_then_by_name() {
        let installed =
            vec![addon("zeta", "Zeta", true, "1"), addon("alpha", "Alpha", true, "1"), addon("mid", "Mid", true, "1")];
        let mut releases = HashMap::new();
        releases.insert("zeta".to_string(), release("2"));
        releases.insert("alpha".to_string(), release("1"));
        assert_eq!(listed_names(&installed, &releases, ""), ["zeta", "alpha", "mid"]);
        let flags: Vec<_> = filtered_addons(&installed, &releases, "").iter().map(|l| l.has_update).collect();
        assert_eq!(flags, [true, false, false]);
    }

    #[test]
    fn unmanaged_never_counts_as_update() {
        let installed = vec![addon("b", "B", false, ""), addon("a", "A", true, "1")];
        let mut releases = HashMap::new();
        releases.insert("b".to_string(), release("9"));
        assert_eq!(listed_names(&installed, &releases, ""), ["a", "b"]);
    }

    #[test]
    fn search_matches_name_alias_description_author() {
        let mut installed = vec![addon("dbm", "Deadly Boss Mods", true, "1"), addon("bag", "Bagnon", true, "1")];
        installed[1].author = "Tuller".into();
        let r = HashMap::new();
        assert_eq!(listed_names(&installed, &r, "DEADLY"), ["dbm"]);
        assert_eq!(listed_names(&installed, &r, "bag"), ["bag"]);
        assert_eq!(listed_names(&installed, &r, "desc of").len(), 2);
        assert_eq!(listed_names(&installed, &r, "tuller"), ["bag"]);
        assert_eq!(listed_names(&installed, &r, "   ").len(), 2);
        assert!(listed_names(&installed, &r, "nothing").is_empty());
    }

    #[test]
    fn selection_reconcile() {
        let a = addon("a", "A", true, "1");
        let b = addon("b", "B", true, "1");
        let list = vec![a.clone(), b.clone()];
        assert_eq!(reconcile_selection(None, &list), Some(Some(&a)));
        assert_eq!(reconcile_selection(None, &[]), None);
        assert_eq!(reconcile_selection(Some("a"), &list), None);
        assert_eq!(reconcile_selection(Some("b"), &list), None);
        assert_eq!(reconcile_selection(Some("gone"), &list), Some(Some(&a)));
        assert_eq!(reconcile_selection(Some("b"), &[]), Some(None));
    }
}
