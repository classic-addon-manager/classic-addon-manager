//! The addons page's view of the catalog store, shared with its title bar toolbar, plus the sorting, tag and search
//! filter logic.
//!
//! The state entity is an app-level global ([`catalog`]) because it must outlive the page: coming back keeps the
//! selected tag and the search text. It follows the catalog store and the installed addons, and rebuilds the list
//! when either changes.
use std::{cmp::Ordering, collections::HashSet, sync::Arc};

use cam_core::{
    addon::Addon,
    api::AddonManifest,
    config::{AddonSort, AddonSortKey},
};
use gpui_kit::{App, AppContext as _, Context, Entity, Global, Subscription};

use crate::{
    state::{
        self, Load,
        catalog::{self as store, Catalog, locale_cmp},
    },
    ui::addon_filter::{AddonSearch, is_new_addon},
};

pub const ALL_TAG: &str = "All";

/// A catalog entry, by its index into [`Catalog::addons`], together with whether it is installed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AddonListItem {
    pub source_index: usize,
    pub is_installed: bool,
}

pub struct CatalogState {
    /// True once the first fetch has finished, whether it succeeded or failed.
    pub is_ready: bool,
    /// Search text applied to the list (already debounced).
    search_query: String,
    /// Tag the list is filtered by, [`ALL_TAG`] for no tag filter.
    selected_tag: String,
    /// True while the toolbar's refresh is running.
    pub is_refreshing: bool,
    /// Every entry of `source`: new addons first (newest first), then by name.
    pub addons: Vec<AddonListItem>,
    /// Tags to choose from. Starts with "All".
    pub tags: Vec<String>,
    /// Message of the latest failed fetch, `None` after a successful one.
    pub error: Option<String>,
    /// Bumped whenever `addons`, `error` or `is_ready` change, so views can tell when to rebuild.
    pub rev: u64,
    /// Bumped whenever the search text or the tag changes.
    filter_rev: u64,
    /// The catalog `addons` indexes into. Empty until the first catalog arrives.
    source: Arc<Catalog>,
    /// The installed addons the `is_installed` marks of `addons` were taken from.
    installed: Option<Arc<Vec<Addon>>>,
    _subs: [Subscription; 2],
}

struct CatalogGlobal(Entity<CatalogState>);
impl Global for CatalogGlobal {}

/// The shared catalog entity (created on first use).
pub fn catalog(cx: &mut App) -> Entity<CatalogState> {
    if let Some(g) = cx.try_global::<CatalogGlobal>() {
        return g.0.clone();
    }
    let entity = cx.new(CatalogState::new);
    cx.set_global(CatalogGlobal(entity.clone()));
    entity
}

impl CatalogState {
    fn new(cx: &mut Context<Self>) -> Self {
        let subs = [
            cx.observe(&state::catalog(cx), |this, _, cx| this.sync(cx)),
            cx.observe(&state::addons(cx), |this, _, cx| this.sync(cx)),
        ];
        let mut this = Self {
            is_ready: false,
            search_query: String::new(),
            selected_tag: ALL_TAG.to_string(),
            is_refreshing: false,
            addons: Vec::new(),
            tags: vec![ALL_TAG.to_string()],
            error: None,
            rev: 0,
            filter_rev: 0,
            source: Arc::new(Catalog::new(Vec::new())),
            installed: None,
            _subs: subs,
        };
        this.sync(cx);
        this
    }

    /// Takes over a new catalog, a new fetch state or changed installed marks.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let load = store::load(cx);
        let is_ready = matches!(load, Load::Ready(_) | Load::Failed(_));
        let current = load.ready().cloned();
        let error = store::error(cx).map(|e| e.message());
        let installed = state::addons(cx).read(cx).installed_addons.clone();
        // The marks only change with the installed list, and the addon store replaces that list when it changes
        let marks_stale = !self.installed.as_ref().is_some_and(|i| Arc::ptr_eq(i, &installed));
        let names = || installed.iter().map(|a| a.name.as_str()).collect::<HashSet<&str>>();
        let mut changed = is_ready != self.is_ready || error != self.error;
        match current {
            Some(catalog) if !Arc::ptr_eq(&self.source, &catalog) => {
                self.addons = build_list(catalog.addons(), &names());
                self.tags = std::iter::once(ALL_TAG.to_string()).chain(catalog.tags().iter().cloned()).collect();
                self.source = catalog;
                changed = true;
            }
            _ if marks_stale => {
                let (names, manifests) = (names(), self.source.addons());
                for item in &mut self.addons {
                    let now = names.contains(manifests[item.source_index].name.as_str());
                    changed |= item.is_installed != now;
                    item.is_installed = now;
                }
            }
            _ => {}
        }
        self.installed = Some(installed);
        if changed {
            self.is_ready = is_ready;
            self.error = error;
            self.rev += 1;
            cx.notify();
        }
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn set_search_query(&mut self, text: String, cx: &mut Context<Self>) {
        if self.search_query != text {
            self.search_query = text;
            self.filter_rev += 1;
            cx.notify();
        }
    }

    pub fn selected_tag(&self) -> &str {
        &self.selected_tag
    }

    pub fn set_selected_tag(&mut self, tag: String, cx: &mut Context<Self>) {
        if self.selected_tag != tag {
            self.selected_tag = tag;
            self.filter_rev += 1;
            cx.notify();
        }
    }

    /// Bumped whenever the search text or the tag changes.
    pub fn filter_rev(&self) -> u64 {
        self.filter_rev
    }

    /// The catalog the entries of `addons` index into.
    pub fn source(&self) -> &Arc<Catalog> {
        &self.source
    }

    /// Indices into `addons` of the entries matching the tag and search text, in `sort` order.
    pub fn filtered_indices(&self, sort: AddonSort) -> Vec<usize> {
        filtered_indices(self.source.addons(), &self.addons, &self.selected_tag, &self.search_query, sort)
    }
}

/// Indices into `addons` (entries of `manifests`) of the entries matching `tag` and `query`, in `sort` order
/// ([`AddonSortKey::Default`] keeps the list order).
pub fn filtered_indices(
    manifests: &[AddonManifest],
    addons: &[AddonListItem],
    tag: &str,
    query: &str,
    sort: AddonSort,
) -> Vec<usize> {
    let search = AddonSearch::new(query);
    let manifest = |i: usize| &manifests[addons[i].source_index];
    let mut indices: Vec<usize> = (0..addons.len()).filter(|&i| matches_filter(manifest(i), tag, &search)).collect();
    if sort.key() != AddonSortKey::Default {
        indices.sort_by(|&a, &b| compare_addons(manifest(a), manifest(b), sort));
    }
    indices
}

/// Lower bound of the 95% Wilson score interval of the like ratio: one like out of one vote ranks below 95 out of
/// 100. `None` for unrated addons. Without a vote count (catalogs served before the API reported it) the plain
/// ratio is used.
pub fn rating_score(m: &AddonManifest) -> Option<f64> {
    const Z: f64 = 1.96;
    let p = f64::from(m.like_percent?) / 100.0;
    let n = match m.votes {
        Some(n) if n > 0 => n as f64,
        Some(_) => return None,
        None => return Some(p),
    };
    let z2 = Z * Z;
    Some((p + z2 / (2.0 * n) - Z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt()) / (1.0 + z2 / n))
}

/// Order of two addons under a non-default `sort`. Unrated addons stay last in both directions, ties fall back to
/// most downloads, then the displayed name.
pub fn compare_addons(a: &AddonManifest, b: &AddonManifest, sort: AddonSort) -> Ordering {
    let by_name = || locale_cmp(&a.alias, &b.alias);
    let primary = match sort.key() {
        AddonSortKey::Default => Ordering::Equal,
        AddonSortKey::Name => by_name(),
        AddonSortKey::Downloads => a.downloads.cmp(&b.downloads),
        AddonSortKey::Rating => match (rating_score(a), rating_score(b)) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
    };
    let primary = if sort.descending() { primary.reverse() } else { primary };
    primary.then_with(|| b.downloads.cmp(&a.downloads)).then_with(by_name)
}

/// Whether an entry passes the selected tag and the search text.
pub fn matches_filter(m: &AddonManifest, selected_tag: &str, search: &AddonSearch) -> bool {
    (selected_tag == ALL_TAG || m.tags.iter().any(|t| t == selected_tag))
        && search.matches(&m.name, &m.alias, &m.description, &m.author)
}

/// Every entry of `manifests` in list order: new addons first (newest first), then by name.
pub fn build_list(manifests: &[AddonManifest], installed: &HashSet<&str>) -> Vec<AddonListItem> {
    let mut items: Vec<AddonListItem> = manifests
        .iter()
        .enumerate()
        .map(|(source_index, m)| AddonListItem { source_index, is_installed: installed.contains(m.name.as_str()) })
        .collect();
    items.sort_by(|a, b| {
        let (a, b) = (&manifests[a.source_index], &manifests[b.source_index]);
        match (is_new_addon(a.added_at), is_new_addon(b.added_at)) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (true, true) if a.added_at != b.added_at => return b.added_at.cmp(&a.added_at),
            _ => {}
        }
        locale_cmp(&a.name, &b.name)
    });
    items
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;

    fn manifest(name: &str, age_days: i64) -> AddonManifest {
        AddonManifest {
            name: name.into(),
            alias: name.to_uppercase(),
            added_at: Some(Utc::now() - Duration::days(age_days)),
            ..Default::default()
        }
    }

    #[test]
    fn new_addons_come_first_newest_first_then_by_name() {
        let manifests =
            vec![manifest("zeta", 200), manifest("alpha", 200), manifest("older-new", 20), manifest("newest", 1)];
        let items = build_list(&manifests, &HashSet::from(["alpha"]));
        let names: Vec<&str> = items.iter().map(|i| manifests[i.source_index].name.as_str()).collect();
        assert_eq!(names, vec!["newest", "older-new", "alpha", "zeta"]);
        assert!(items[2].is_installed && !items[3].is_installed);
    }

    #[test]
    fn filter_needs_tag_and_text() {
        let mut m = manifest("bagpack", 100);
        m.tags = vec!["UI".into()];
        m.author = "Madpeter".into();
        let matches = |tag: &str, text: &str| matches_filter(&m, tag, &AddonSearch::new(text));
        assert!(matches(ALL_TAG, ""));
        assert!(matches("UI", "Madpe"));
        assert!(!matches("Raid", ""));
        assert!(!matches("UI", "zzz"));
    }

    fn rated(alias: &str, downloads: i64, like: Option<i8>, votes: Option<i64>) -> AddonManifest {
        AddonManifest {
            name: alias.to_lowercase(),
            alias: alias.into(),
            downloads,
            like_percent: like,
            votes,
            added_at: Some(Utc::now() - Duration::days(100)),
            ..Default::default()
        }
    }

    /// Aliases of `manifests` filtered without a tag or text, listed in load order.
    fn order(manifests: Vec<AddonManifest>, sort: AddonSort) -> Vec<String> {
        let items: Vec<AddonListItem> =
            (0..manifests.len()).map(|source_index| AddonListItem { source_index, is_installed: false }).collect();
        filtered_indices(&manifests, &items, ALL_TAG, "", sort)
            .into_iter()
            .map(|i| manifests[items[i].source_index].alias.clone())
            .collect()
    }

    #[test]
    fn rating_weighs_vote_count_and_keeps_unrated_last() {
        let addons = || {
            vec![
                rated("Unrated", 900, None, Some(0)),
                rated("OneLike", 10, Some(100), Some(1)),
                rated("Popular", 20, Some(95), Some(100)),
                rated("Disliked", 30, Some(20), Some(10)),
                rated("Legacy", 5, Some(50), None),
            ]
        };
        assert_eq!(order(addons(), AddonSort::RatingDesc), ["Popular", "Legacy", "OneLike", "Disliked", "Unrated"]);
        assert_eq!(order(addons(), AddonSort::RatingAsc), ["Disliked", "OneLike", "Legacy", "Popular", "Unrated"]);
    }

    #[test]
    fn ties_fall_back_to_downloads_then_displayed_name() {
        let addons = vec![
            rated("beta", 5, Some(100), Some(4)),
            rated("Alpha", 5, Some(100), Some(4)),
            rated("Gamma", 50, Some(100), Some(4)),
        ];
        assert_eq!(order(addons, AddonSort::RatingDesc), ["Gamma", "Alpha", "beta"]);
    }

    #[test]
    fn name_and_downloads_follow_direction() {
        let addons = || vec![rated("b", 1, None, None), rated("C", 3, None, None), rated("a", 2, None, None)];
        assert_eq!(order(addons(), AddonSort::NameAsc), ["a", "b", "C"]);
        assert_eq!(order(addons(), AddonSort::NameDesc), ["C", "b", "a"]);
        assert_eq!(order(addons(), AddonSort::DownloadsDesc), ["C", "a", "b"]);
        // the default order is the load order (already new first, then by name)
        assert_eq!(order(addons(), AddonSort::default()), ["b", "C", "a"]);
    }
}
