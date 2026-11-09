//! The addon catalog (`GET /addons`), held once for the whole app.
//!
//! Views observe [`super::catalog`], read [`load`] in `render` and call [`ensure`] when they show the catalog. The
//! Refresh button and a sign-in change fetch it again ([`refresh`]). Code that continues with the catalog awaits
//! [`fetch`] and passes it on, for example to dependency resolution. Whether an addon is installed comes from
//! [`AddonState::is_installed`](super::AddonState::is_installed).
use std::{cmp::Ordering, collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context as _;
use cam_core::api::{self, AddonManifest};
use gpui_kit::{App, Task};

use super::query::{Load, Query, QueryError};

/// A catalog fetched less than this long ago is used without asking the server again.
const STALE: Duration = Duration::from_secs(5 * 60);

/// The fetched catalog with lookups by addon name.
pub struct Catalog {
    addons: Vec<AddonManifest>,
    by_name: HashMap<String, usize>,
    tags: Vec<String>,
    max_downloads: i64,
}

impl Catalog {
    pub fn new(addons: Vec<AddonManifest>) -> Self {
        let by_name = addons.iter().enumerate().map(|(i, m)| (m.name.clone(), i)).collect();
        let mut tags: Vec<String> = addons.iter().flat_map(|m| &m.tags).filter(|t| *t != "Example").cloned().collect();
        tags.sort_by(|a, b| locale_cmp(a, b));
        tags.dedup();
        let max_downloads = addons.iter().map(|m| m.downloads).max().unwrap_or(0).max(0);
        Self { addons, by_name, tags, max_downloads }
    }

    /// Every entry, in the order the server sent them.
    pub fn addons(&self) -> &[AddonManifest] {
        &self.addons
    }

    pub fn manifest(&self, name: &str) -> Option<&AddonManifest> {
        self.by_name.get(name).map(|&i| &self.addons[i])
    }

    /// The icon of `name`, `None` when the addon is unknown or has no icon.
    pub fn icon_url(&self, name: &str) -> Option<&str> {
        self.manifest(name)?.icon.as_deref().filter(|url| !url.is_empty())
    }

    /// Every tag in use except "Example", sorted by [`locale_cmp`].
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Highest download count of any entry.
    pub fn max_downloads(&self) -> i64 {
        self.max_downloads
    }
}

pub type CatalogQuery = Query<(), Arc<Catalog>>;

pub(super) fn query() -> CatalogQuery {
    Query::blocking(
        |_| "the addon catalog".into(),
        STALE,
        |_| Ok(Arc::new(Catalog::new(api::catalog().context("fetch addon manifest")?))),
    )
}

pub fn load(cx: &App) -> &Load<Arc<Catalog>> {
    super::catalog(cx).read(cx).get(&())
}

/// The catalog when one was fetched, also while it is fetched again or after a failed refetch.
pub fn current(cx: &App) -> Option<Arc<Catalog>> {
    load(cx).ready().cloned()
}

/// Failure of the latest fetch, also when an older catalog is kept.
pub fn error(cx: &App) -> Option<QueryError> {
    super::catalog(cx).read(cx).error(&()).cloned()
}

/// Fetches the catalog unless it is fresh or already being fetched.
pub fn ensure(cx: &mut App) {
    super::catalog(cx).update(cx, |q, cx| q.ensure((), cx));
}

/// The catalog for work that needs one: the fresh one, or the result of a fetch. When that fetch fails, the catalog
/// fetched before is used, so a server hiccup doesn't block installs. Fails only when no catalog was ever fetched.
pub fn fetch(cx: &mut App) -> Task<Result<Arc<Catalog>, QueryError>> {
    let query = super::catalog(cx);
    let fetch = query.update(cx, |q, cx| q.fetch((), cx));
    cx.spawn(async move |cx| match fetch.await {
        Err(e) => cx.update(|cx| query.read(cx).value(&()).cloned()).ok_or(e),
        fetched => fetched,
    })
}

/// Fetches the catalog from the server even when it is fresh.
pub fn refresh(cx: &mut App) -> Task<Result<Arc<Catalog>, QueryError>> {
    super::catalog(cx).update(cx, |q, cx| q.refetch((), cx))
}

/// Compares names the way a person sorts them: punctuation before digits before letters, letters compared
/// case-insensitively first, and on ties lower case before upper case.
pub fn locale_cmp(a: &str, b: &str) -> Ordering {
    fn key(c: char) -> (u8, char) {
        if c.is_alphabetic() {
            (2, c)
        } else if c.is_numeric() {
            (1, c)
        } else {
            (0, c)
        }
    }
    let ka = a.chars().flat_map(char::to_lowercase).map(key);
    let kb = b.chars().flat_map(char::to_lowercase).map(key);
    ka.cmp(kb).then_with(|| b.cmp(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str, tags: &[&str], downloads: i64) -> AddonManifest {
        AddonManifest {
            name: name.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            downloads,
            ..Default::default()
        }
    }

    #[test]
    fn locale_compare_is_case_insensitive_with_lower_first() {
        assert_eq!(locale_cmp("alpha", "Beta"), Ordering::Less);
        assert_eq!(locale_cmp("Beta", "alpha"), Ordering::Greater);
        assert_eq!(locale_cmp("a", "A"), Ordering::Less);
        assert_eq!(locale_cmp("same", "same"), Ordering::Equal);
        assert_eq!(locale_cmp("a-b", "ab"), Ordering::Less);
        assert_eq!(locale_cmp("a1", "aa"), Ordering::Less);
        assert_eq!(locale_cmp("_x", "1x"), Ordering::Less);
        assert_eq!(locale_cmp("ab", "abc"), Ordering::Less);
    }

    #[test]
    fn tags_are_unique_sorted_and_skip_example() {
        let c = Catalog::new(vec![manifest("a", &["UI", "Example"], 0), manifest("b", &["bags", "UI"], 0)]);
        assert_eq!(c.tags(), ["bags", "UI"]);
    }

    #[test]
    fn lookups_by_name() {
        let mut with_icon = manifest("b", &[], 9);
        with_icon.icon = Some("https://x/icon.png".into());
        let mut empty_icon = manifest("c", &[], -1);
        empty_icon.icon = Some(String::new());
        let c = Catalog::new(vec![manifest("a", &[], 3), with_icon, empty_icon]);
        assert_eq!(c.manifest("b").map(|m| m.downloads), Some(9));
        assert!(c.manifest("missing").is_none());
        assert_eq!(c.icon_url("b"), Some("https://x/icon.png"));
        assert_eq!(c.icon_url("a"), None);
        assert_eq!(c.icon_url("c"), None);
        assert_eq!(c.max_downloads(), 9);
        assert_eq!(Catalog::new(vec![manifest("z", &[], -5)]).max_downloads(), 0);
    }
}
