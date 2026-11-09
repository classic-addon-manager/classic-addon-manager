//! Cache for the developer page: values of a declaration, submission details, addon statistics and the owner's list of
//! addons and submissions. It is one global entity shared by all views, so it survives page switches, and its entries
//! are keyed by the signed-in Discord id.
//!
//! Every kind of lookup is a [`Query`] entity held here, and this entity notifies when one of them or the addon catalog
//! changes. Views `cx.observe(&queries, ..)` the entity, read state with the `values()/submission()/stats()/owned()`
//! getters in `render`, and call the matching `ensure_*` method once when they are created. That call fetches again every
//! time a view is created (cached data stays visible meanwhile), except for statistics, which are only fetched again
//! after 15 minutes. A lookup that failed is fetched again by the next call whatever its age.
use std::{collections::HashSet, hash::Hash, time::Duration};

use cam_core::api::developer::{self, types::*};
use cam_core::api::{AddonManifest, ApiError};
use cam_ui::toast;
use gpui_kit::{App, AppContext, Context, Entity, Global, Subscription, Task};

use super::owned::{self, OwnedAddonsData};
use crate::state::{self, Load, Query, QueryError};

/// Statistics snapshots are taken hourly, so statistics fetched less than 15 minutes ago are not fetched again when a
/// view is created.
const STATS_STALE: Duration = Duration::from_secs(15 * 60);

/// What a view sees for the values of one declaration source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValuesView {
    pub values: Option<DeclarationValues>,
    pub kind: Option<DeclarationKind>,
    pub error: Option<String>,
    /// True while a source is given and nothing has been fetched for it yet.
    pub loading: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubmissionView {
    pub submission: Option<SubmissionDetail>,
    pub error: Option<String>,
    pub loading: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatsView {
    pub stats: Option<AddonDeveloperStats>,
    pub error: Option<String>,
    pub loading: bool,
}

/// What the owned list sees.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OwnedView {
    pub data: Option<OwnedAddonsData>,
    /// Only set when there is nothing to display (poll failures keep the last data and stay silent).
    pub error: Option<String>,
    pub last_attempt_failed: bool,
}

/// Declaration values by `(discord id, source)`.
type ValuesQuery = Query<(String, EditorSource), SourceValues>;
/// Submission details by `(discord id, submission id)`.
type SubmissionQuery = Query<(String, i64), SubmissionDetail>;
/// Statistics by `(discord id, addon uuid)`.
type StatsQuery = Query<(String, String), AddonDeveloperStats>;
/// The owner's addons and submissions as the server lists them, by discord id.
type OwnedQuery = Query<String, AddonSources>;
/// The declaration schema, fetched once and then kept for the rest of the run.
type SchemaQuery = Query<(), SchemaDoc>;

/// What views asked for since the page was opened, which is what [`DevQueries::retry`] fetches again.
#[derive(Default)]
struct Touched {
    values: HashSet<EditorSource>,
    submissions: HashSet<i64>,
    stats: HashSet<String>,
}

pub struct DevQueries {
    schema: Entity<SchemaQuery>,
    values: Entity<ValuesQuery>,
    submissions: Entity<SubmissionQuery>,
    stats: Entity<StatsQuery>,
    owned: Entity<OwnedQuery>,
    touched: Touched,
    _subs: Vec<Subscription>,
}

struct GlobalQueries(Entity<DevQueries>);
impl Global for GlobalQueries {}

fn discord_id(cx: &App) -> String {
    state::user(cx).read(cx).user.discord_id.clone()
}

/// The error of a failed request: the sentence `messages` has for it on top, with the [`ApiError`] below. Views show the
/// sentence and the log keeps both.
fn failure(messages: &developer::Messages, error: ApiError) -> anyhow::Error {
    let message = messages.message(&error);
    anyhow::Error::new(error).context(message)
}

/// What a view shows for one key. A failed latest fetch hides an older value, because the view then shows the failure
/// and not data that may be outdated.
fn latest<'a, V>(load: &'a Load<V>, error: Option<&QueryError>) -> Load<&'a V> {
    match error {
        Some(e) => Load::Failed(e.clone()),
        None => load.as_ref(),
    }
}

/// What the owned list sees. A failure only shows as `error` while there is no list. After a failed refetch the last
/// list stays and `last_attempt_failed` is set.
fn owned_view(sources: Option<&AddonSources>, manifests: &[AddonManifest], error: Option<&QueryError>) -> OwnedView {
    let data = sources.map(|sources| owned::sources_to_owned(sources, manifests));
    OwnedView {
        error: error.filter(|_| data.is_none()).map(ToString::to_string),
        last_attempt_failed: error.is_some(),
        data,
    }
}

fn schema_query() -> SchemaQuery {
    SchemaQuery::blocking(
        |_| "the declaration schema".into(),
        Duration::MAX,
        |_| developer::schema().map_err(|e| failure(&developer::SCHEMA, e)),
    )
}

/// Values can't be read without a schema, so the fetch waits for it first. A schema that can't be fetched or that this
/// app can't use reads as an unavailable source.
fn values_query(schema: Entity<SchemaQuery>) -> ValuesQuery {
    ValuesQuery::new(
        |(_, source)| match source {
            EditorSource::New => "the values of a new addon".to_owned(),
            EditorSource::Addon { uuid } => format!("the values of addon {uuid}"),
            EditorSource::Submission { id } => format!("the values of submission {id}"),
        },
        Duration::ZERO,
        move |(_, source), cx| {
            let schema = schema.update(cx, |q, cx| q.fetch((), cx));
            let source = source.clone();
            cx.spawn(async move |cx| {
                let Ok(SchemaDoc::Supported(schema)) = schema.await else {
                    return Err(failure(&developer::VALUES, ApiError::NotFound));
                };
                cx.background_executor()
                    .spawn(
                        async move { developer::values(&source, &schema).map_err(|e| failure(&developer::VALUES, e)) },
                    )
                    .await
            })
        },
    )
}

/// Fetches every key again and returns the running fetches.
fn refetch_all<K, V>(
    query: &Entity<Query<K, V>>,
    keys: impl Iterator<Item = K>,
    cx: &mut Context<DevQueries>,
) -> Vec<Task<Result<V, QueryError>>>
where
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
{
    keys.map(|key| query.update(cx, |q, cx| q.refetch(key, cx))).collect()
}

/// Waits until every fetch is done. A failure is shown by the view that reads that lookup, so it is not reported here.
async fn finish<V: 'static>(fetches: Vec<Task<Result<V, QueryError>>>) {
    for fetch in fetches {
        let _ = fetch.await;
    }
}

impl DevQueries {
    pub fn global(cx: &mut App) -> Entity<DevQueries> {
        if let Some(g) = cx.try_global::<GlobalQueries>() {
            return g.0.clone();
        }
        let entity = cx.new(Self::new);
        cx.set_global(GlobalQueries(entity.clone()));
        entity
    }

    fn new(cx: &mut Context<Self>) -> Self {
        let schema = cx.new(|_| schema_query());
        let values = {
            let schema = schema.clone();
            cx.new(|_| values_query(schema))
        };
        let submissions = cx.new(|_| {
            SubmissionQuery::blocking(
                |(_, id)| format!("submission {id}"),
                Duration::ZERO,
                |(_, id)| developer::submission(*id).map_err(|e| failure(&developer::SUBMISSION, e)),
            )
        });
        let stats = cx.new(|_| {
            StatsQuery::blocking(
                |(_, uuid)| format!("the statistics of addon {uuid}"),
                STATS_STALE,
                |(_, uuid)| developer::stats(uuid).map_err(|e| failure(&developer::STATS, e)),
            )
        });
        let owned = cx.new(|_| {
            OwnedQuery::blocking(
                |_| "your addons and submissions".into(),
                Duration::ZERO,
                |_| developer::sources().map_err(|e| failure(&developer::SOURCES, e)),
            )
        });
        // The owned list shows details from the catalog, so it changes with the catalog too.
        let subs = vec![
            cx.observe(&schema, |_, _, cx| cx.notify()),
            cx.observe(&values, |_, _, cx| cx.notify()),
            cx.observe(&submissions, |_, _, cx| cx.notify()),
            cx.observe(&stats, |_, _, cx| cx.notify()),
            cx.observe(&owned, |_, _, cx| cx.notify()),
            cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()),
        ];
        Self { schema, values, submissions, stats, owned, touched: Touched::default(), _subs: subs }
    }

    /// The declaration schema: the one fetched before, or the result of a fetch.
    pub fn fetch_schema(cx: &mut App) -> Task<Result<SchemaDoc, QueryError>> {
        let schema = Self::global(cx).read(cx).schema.clone();
        schema.update(cx, |q, cx| q.fetch((), cx))
    }

    /// Called when a new Developer page is created: only queries touched afterwards count as active.
    pub fn begin_session(&mut self) {
        self.touched = Touched::default();
    }

    // ---- values ---------------------------------------------------------------------------------------------------

    pub fn values(&self, source: Option<&EditorSource>, cx: &App) -> ValuesView {
        let Some(source) = source else { return ValuesView::default() };
        let query = self.values.read(cx);
        let key = (discord_id(cx), source.clone());
        match latest(query.get(&key), query.error(&key)) {
            Load::Ready(v) => ValuesView { values: Some(v.values.clone()), kind: Some(v.kind), ..Default::default() },
            Load::Failed(e) => ValuesView { error: Some(e.to_string()), ..Default::default() },
            Load::Idle | Load::Pending => ValuesView { loading: true, ..Default::default() },
        }
    }

    pub fn ensure_values(&mut self, source: &EditorSource, cx: &mut Context<Self>) {
        self.touched.values.insert(source.clone());
        let key = (discord_id(cx), source.clone());
        self.values.update(cx, |q, cx| q.ensure(key, cx));
    }

    /// Fetches the values again even when they were fetched recently.
    pub fn refetch_values(&mut self, source: &EditorSource, cx: &mut Context<Self>) {
        self.touched.values.insert(source.clone());
        let key = (discord_id(cx), source.clone());
        self.values.update(cx, |q, cx| q.refetch(key, cx).detach());
    }

    // ---- submission detail ----------------------------------------------------------------------------------------

    pub fn submission(&self, id: i64, cx: &App) -> SubmissionView {
        let query = self.submissions.read(cx);
        let key = (discord_id(cx), id);
        match latest(query.get(&key), query.error(&key)) {
            Load::Ready(s) => SubmissionView { submission: Some(s.clone()), ..Default::default() },
            Load::Failed(e) => SubmissionView { error: Some(e.to_string()), ..Default::default() },
            Load::Idle | Load::Pending => SubmissionView { loading: true, ..Default::default() },
        }
    }

    pub fn ensure_submission(&mut self, id: i64, cx: &mut Context<Self>) {
        self.touched.submissions.insert(id);
        let key = (discord_id(cx), id);
        self.submissions.update(cx, |q, cx| q.ensure(key, cx));
    }

    // ---- stats ----------------------------------------------------------------------------------------------------

    pub fn stats(&self, uuid: &str, cx: &App) -> StatsView {
        let query = self.stats.read(cx);
        let key = (discord_id(cx), uuid.to_owned());
        match latest(query.get(&key), query.error(&key)) {
            Load::Ready(s) => StatsView { stats: Some(s.clone()), ..Default::default() },
            Load::Failed(e) => StatsView { error: Some(e.to_string()), ..Default::default() },
            Load::Idle | Load::Pending => StatsView { loading: true, ..Default::default() },
        }
    }

    pub fn ensure_stats(&mut self, uuid: &str, cx: &mut Context<Self>) {
        self.touched.stats.insert(uuid.to_owned());
        let key = (discord_id(cx), uuid.to_owned());
        self.stats.update(cx, |q, cx| q.ensure(key, cx));
    }

    pub fn refetch_stats(&mut self, uuid: &str, cx: &mut Context<Self>) {
        self.touched.stats.insert(uuid.to_owned());
        let key = (discord_id(cx), uuid.to_owned());
        self.stats.update(cx, |q, cx| q.refetch(key, cx).detach());
    }

    // ---- owned addons ---------------------------------------------------------------------------------------------

    pub fn owned(&self, cx: &App) -> OwnedView {
        let id = discord_id(cx);
        let query = self.owned.read(cx);
        // The details from the catalog are best effort: the list shows without them until the catalog is there.
        let catalog = state::catalog::current(cx);
        let manifests = catalog.as_deref().map(|c| c.addons()).unwrap_or_default();
        owned_view(query.value(&id), manifests, query.error(&id))
    }

    /// Fetches the owner's list (page open, 60 s poll) unless a fetch is running, and the catalog that adds details to
    /// it unless that is fresh. Silent: failures only surface through [`Self::owned`].
    pub fn fetch_owned(&mut self, cx: &mut Context<Self>) {
        state::catalog::ensure(cx);
        let id = discord_id(cx);
        self.owned.update(cx, |q, cx| q.ensure(id, cx));
    }

    /// `retry`: fetches everything the page touched (statistics, submission detail, values) again, then the list. A
    /// failing user-triggered refresh with data on screen toasts the error.
    pub fn retry(this: &Entity<DevQueries>, cx: &mut App) -> Task<()> {
        let this = this.clone();
        cx.spawn(async move |cx| {
            let (values, submissions, stats) = this.update(cx, |q, cx| {
                let id = discord_id(cx);
                (
                    refetch_all(&q.values, q.touched.values.iter().map(|s| (id.clone(), s.clone())), cx),
                    refetch_all(&q.submissions, q.touched.submissions.iter().map(|&n| (id.clone(), n)), cx),
                    refetch_all(&q.stats, q.touched.stats.iter().map(|u| (id.clone(), u.clone())), cx),
                )
            });
            finish(values).await;
            finish(submissions).await;
            finish(stats).await;
            let (had_data, owned) = this.update(cx, |q, cx| {
                state::catalog::ensure(cx);
                let id = discord_id(cx);
                let had_data = q.owned.read(cx).value(&id).is_some();
                (had_data, q.owned.update(cx, |q, cx| q.refetch(id, cx)))
            });
            // Poll failures stay silent, a user-triggered retry surfaces the error.
            if let (true, Err(e)) = (had_data, owned.await) {
                cx.update(|cx| {
                    toast::error(cx, e.to_string());
                });
            }
        })
    }

    /// Drops a submission from the cached list.
    pub fn remove_submission(&mut self, id: i64, cx: &mut Context<Self>) {
        let user = discord_id(cx);
        self.owned.update(cx, |q, cx| q.update(&user, |sources| sources.submissions.retain(|s| s.id != id), cx));
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    fn down() -> QueryError {
        QueryError::from(anyhow!("down"))
    }

    #[test]
    fn a_failure_shows_the_sentence_of_its_endpoint_and_keeps_the_api_error() {
        let error = failure(&developer::STATS, ApiError::NotFound);
        assert_eq!(QueryError::from(error).to_string(), "These statistics are unavailable.");

        let error = failure(&developer::VALUES, ApiError::Network("timed out".into()));
        assert_eq!(error.downcast_ref::<ApiError>(), Some(&ApiError::Network("timed out".into())));
        assert_eq!(QueryError::from(error).to_string(), "Unexpected values response.");
    }

    #[test]
    fn a_failed_latest_fetch_hides_an_older_value() {
        let value = Load::Ready(3);
        assert!(matches!(latest(&value, None), Load::Ready(&3)));
        assert!(matches!(latest(&value, Some(&down())), Load::Failed(_)));
        assert!(matches!(latest(&Load::<i32>::Pending, None), Load::Pending));
        assert!(matches!(latest(&Load::<i32>::Idle, None), Load::Idle));
    }

    #[test]
    fn the_owned_list_shows_a_failure_only_when_there_is_no_list() {
        assert_eq!(owned_view(None, &[], None), OwnedView::default());

        let first_fetch_failed = owned_view(None, &[], Some(&down()));
        assert_eq!(first_fetch_failed.data, None);
        assert_eq!(first_fetch_failed.error.as_deref(), Some("down"));
        assert!(first_fetch_failed.last_attempt_failed);

        let sources = AddonSources::default();
        let refetch_failed = owned_view(Some(&sources), &[], Some(&down()));
        assert!(refetch_failed.data.is_some());
        assert_eq!(refetch_failed.error, None);
        assert!(refetch_failed.last_attempt_failed);
    }
}
