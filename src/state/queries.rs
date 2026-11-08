//! App-wide lookups of addon details, one [`Query`] entity each: the latest release, the README and every published
//! version of an addon. Views observe the entity they read, see [`super::query`].
use std::time::Duration;

use gpui_kit::{App, AppContext as _, Entity};

use super::{catalog, query::Query, ratings};
use crate::backend::{
    api::{self, Release},
    shared::AddonManifest,
};

/// Release and README lookups younger than this are not fetched again.
const STALE: Duration = Duration::from_secs(5 * 60);

/// The latest release, by addon name.
pub type ReleaseQuery = Query<String, Release>;
/// README.md of a GitHub repository, by `(repo, branch)`. `None` when the repository has none.
pub type ReadmeQuery = Query<(String, String), Option<String>>;
/// Every published release, by addon name, in the order the server sends them.
pub type VersionsQuery = Query<String, Vec<Release>>;

pub(super) struct Queries {
    release: Entity<ReleaseQuery>,
    readme: Entity<ReadmeQuery>,
    versions: Entity<VersionsQuery>,
    rating: Entity<ratings::RatingQuery>,
}

impl Queries {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            release: cx.new(|_| {
                ReleaseQuery::blocking(
                    |name| format!("the latest release of {name}"),
                    STALE,
                    |name| Ok(api::release(name, "latest")?),
                )
            }),
            readme: cx.new(|_| {
                ReadmeQuery::blocking(
                    |(repo, branch)| format!("the README of {repo}@{branch}"),
                    STALE,
                    |(repo, branch)| Ok(api::readme(repo, branch)?),
                )
            }),
            // The version picker always asks the server, and shows the last list meanwhile.
            versions: cx.new(|_| {
                VersionsQuery::blocking(
                    |name| format!("the versions of {name}"),
                    Duration::ZERO,
                    |name| Ok(api::releases(name)?),
                )
            }),
            rating: cx.new(|_| ratings::query()),
        }
    }
}

fn queries(cx: &App) -> &Queries {
    &cx.global::<super::AppState>().queries
}

pub fn release(cx: &App) -> Entity<ReleaseQuery> {
    queries(cx).release.clone()
}

pub fn readme(cx: &App) -> Entity<ReadmeQuery> {
    queries(cx).readme.clone()
}

pub fn versions(cx: &App) -> Entity<VersionsQuery> {
    queries(cx).versions.clone()
}

pub(super) fn rating(cx: &App) -> Entity<ratings::RatingQuery> {
    queries(cx).rating.clone()
}

/// The signed-in user changed: every lookup is outdated, and a catalog that was shown is fetched again. Ratings are
/// kept by user, so they stay valid.
pub fn invalidate_all(cx: &mut App) {
    let q = queries(cx);
    let (release, readme, versions) = (q.release.clone(), q.readme.clone(), q.versions.clone());
    release.update(cx, |q, cx| q.invalidate_all(cx));
    readme.update(cx, |q, cx| q.invalidate_all(cx));
    versions.update(cx, |q, cx| q.invalidate_all(cx));
    super::catalog(cx).update(cx, |q, cx| q.invalidate_all(cx));
    if !matches!(catalog::load(cx), super::Load::Idle) {
        catalog::ensure(cx);
    }
}

/// Starts the release and README lookups of `manifest` (when hovering an addon), so its dialog opens with them.
pub fn prefetch_addon_details(cx: &mut App, manifest: &AddonManifest) {
    release(cx).update(cx, |q, cx| q.ensure(manifest.name.clone(), cx));
    if !manifest.repo.is_empty() {
        readme(cx).update(cx, |q, cx| q.ensure((manifest.repo.clone(), manifest.branch.clone()), cx));
    }
}
