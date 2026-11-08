//! Keyed, cached lookups held in a gpui entity, and [`Load`], the state of one lookup.
//!
//! A [`Query`] maps keys to the state of their fetch. Views observe the entity and read [`Query::get`] in `render`,
//! call [`Query::ensure`] when they need a value, and await [`Query::fetch`] when they need the result itself (to
//! toast a failure or to continue with the value). At most one fetch runs per key: callers that arrive while it runs
//! share its result. A value stays visible while it is fetched again and after a failed refetch, [`Query::error`]
//! reports that failure. Every failed fetch is logged once, here.
//!
//! ```ignore
//! let releases = cx.new(|_| Query::blocking(|name| format!("versions of {name}"), STALE, |name: &String| Ok(api::releases(name)?)));
//! releases.update(cx, |q, cx| q.ensure(name.clone(), cx));
//! // render:
//! match releases.read(cx).get(&name) { Load::Ready(list) => .., Load::Failed(e) => e.message(), _ => spinner }
//! ```
use std::{
    collections::HashMap,
    fmt,
    hash::Hash,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use gpui_kit::{App, Context, Task};

use super::tasks::{error_message, run_blocking};

/// State of a lookup.
#[derive(Clone, Debug, Default)]
pub enum Load<T> {
    /// Not requested yet.
    #[default]
    Idle,
    /// The first fetch is running.
    Pending,
    Ready(T),
    /// The fetch failed and no earlier value exists.
    Failed(QueryError),
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Load::Ready(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Load::Pending)
    }

    pub fn as_ref(&self) -> Load<&T> {
        match self {
            Load::Idle => Load::Idle,
            Load::Pending => Load::Pending,
            Load::Ready(v) => Load::Ready(v),
            Load::Failed(e) => Load::Failed(e.clone()),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Load<U> {
        match self {
            Load::Idle => Load::Idle,
            Load::Pending => Load::Pending,
            Load::Ready(v) => Load::Ready(f(v)),
            Load::Failed(e) => Load::Failed(e),
        }
    }
}

/// A failed fetch. Cloning is cheap, so the cached entry and every caller that awaited the fetch share one error.
///
/// `{}` shows the outermost message, `{:#}` the whole cause chain. Converted back into an `anyhow::Error`, the chain
/// below the outermost message keeps its types, so `downcast_ref` finds errors that a fetcher wrapped in context.
#[derive(Clone, Debug)]
pub struct QueryError(Arc<anyhow::Error>);

impl QueryError {
    /// The whole cause chain as a sentence for users.
    pub fn message(&self) -> String {
        error_message(&self.0)
    }
}

impl From<anyhow::Error> for QueryError {
    fn from(e: anyhow::Error) -> Self {
        Self(Arc::new(e))
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() { write!(f, "{:#}", self.0) } else { write!(f, "{}", self.0) }
    }
}

impl std::error::Error for QueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// Logs a failed operation, unless it failed because a [`Query`] fetch failed. The query logged that failure already.
/// The line carries the location of the caller, not of this function.
#[track_caller]
pub fn log_failure(msg: &str, error: &anyhow::Error) {
    if !is_query_failure(error) {
        let caller = std::panic::Location::caller();
        log::logger().log(
            &log::Record::builder()
                .level(log::Level::Error)
                .target(module_path!())
                .file(Some(caller.file()))
                .line(Some(caller.line()))
                .args(format_args!("{msg} {error:#}"))
                .build(),
        );
    }
}

fn is_query_failure(error: &anyhow::Error) -> bool {
    error.chain().any(|e| e.is::<QueryError>())
}

type Fetcher<K, V> = Rc<dyn Fn(&K, &mut App) -> Task<Result<V>>>;
/// Names what is looked up for one key, for the log: "the latest release of DBM".
type Describe<K> = fn(&K) -> String;
type Waiter<V> = async_channel::Sender<Result<V, QueryError>>;

/// Cached lookups keyed by `K`. A value is fresh for `stale` after it was fetched: [`Query::ensure`] and
/// [`Query::fetch`] use a fresh value and fetch again otherwise.
pub struct Query<K, V> {
    describe: Describe<K>,
    stale: Duration,
    fetcher: Fetcher<K, V>,
    entries: HashMap<K, Entry<V>>,
    /// What [`Query::get`] returns for a key that has no entry.
    idle: Load<V>,
}

impl<K, V> Query<K, V>
where
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
{
    /// `fetch` starts the lookup of one key on the main thread and returns the task that resolves to its value.
    pub fn new(
        describe: Describe<K>,
        stale: Duration,
        fetch: impl Fn(&K, &mut App) -> Task<Result<V>> + 'static,
    ) -> Self {
        Self { describe, stale, fetcher: Rc::new(fetch), entries: HashMap::new(), idle: Load::Idle }
    }

    /// A lookup that is one blocking call, run on the background executor.
    pub fn blocking(
        describe: Describe<K>,
        stale: Duration,
        fetch: impl Fn(&K) -> Result<V> + Send + Sync + 'static,
    ) -> Self
    where
        K: Send,
        V: Send,
    {
        let fetch = Arc::new(fetch);
        Self::new(describe, stale, move |key, cx| {
            let (fetch, key) = (fetch.clone(), key.clone());
            run_blocking(cx, move || fetch(&key))
        })
    }

    pub fn get(&self, key: &K) -> &Load<V> {
        self.entries.get(key).map_or(&self.idle, |e| &e.load)
    }

    pub fn value(&self, key: &K) -> Option<&V> {
        self.get(key).ready()
    }

    /// Failure of the latest fetch of `key`: the error of [`Load::Failed`], or of a refetch that kept an older value.
    pub fn error(&self, key: &K) -> Option<&QueryError> {
        let entry = self.entries.get(key)?;
        match &entry.load {
            Load::Failed(e) => Some(e),
            _ => entry.refetch_error.as_ref(),
        }
    }

    /// Fetches `key` unless its value is fresh or a fetch is already running.
    pub fn ensure(&mut self, key: K, cx: &mut Context<Self>) {
        let entry = self.entries.entry(key.clone()).or_default();
        if !entry.is_fresh(self.stale) && entry.task.is_none() {
            self.start(key, cx);
        }
    }

    /// The value of `key`: the fresh one, the result of the running fetch, or of a new fetch.
    pub fn fetch(&mut self, key: K, cx: &mut Context<Self>) -> Task<Result<V, QueryError>> {
        let entry = self.entries.entry(key.clone()).or_default();
        if entry.is_fresh(self.stale)
            && let Load::Ready(v) = &entry.load
        {
            return Task::ready(Ok(v.clone()));
        }
        let waiter = entry.wait();
        if entry.task.is_none() {
            self.start(key, cx);
        }
        waiter.receive(cx)
    }

    /// Fetches `key` again even when its value is fresh, replacing a fetch that is already running.
    pub fn refetch(&mut self, key: K, cx: &mut Context<Self>) -> Task<Result<V, QueryError>> {
        let waiter = self.entries.entry(key.clone()).or_default().wait();
        self.start(key, cx);
        waiter.receive(cx)
    }

    /// Stores `value` as freshly fetched. A running fetch is dropped, its callers receive `value`.
    pub fn set(&mut self, key: K, value: V, cx: &mut Context<Self>) {
        self.entries.entry(key).or_default().put(value);
        cx.notify();
    }

    /// Changes the value of `key` in place, when there is one.
    pub fn update(&mut self, key: &K, f: impl FnOnce(&mut V), cx: &mut Context<Self>) {
        if let Some(Entry { load: Load::Ready(v), .. }) = self.entries.get_mut(key) {
            f(v);
            cx.notify();
        }
    }

    /// Marks every value as outdated, so the next [`Query::ensure`] or [`Query::fetch`] fetches it again. Running
    /// fetches restart, because they may have been sent before whatever made the values outdated; their callers
    /// receive the new result.
    pub fn invalidate_all(&mut self, cx: &mut Context<Self>) {
        let running: Vec<K> = self
            .entries
            .iter_mut()
            .filter_map(|(key, entry)| {
                entry.fetched_at = None;
                entry.task.is_some().then(|| key.clone())
            })
            .collect();
        for key in running {
            self.start(key, cx);
        }
    }

    fn start(&mut self, key: K, cx: &mut Context<Self>) {
        let fetch = (self.fetcher.clone())(&key, cx);
        let entry = self.entries.entry(key.clone()).or_default();
        let generation = entry.begin();
        // Replacing the task of an earlier fetch drops it, which cancels it.
        entry.task = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await.map_err(QueryError::from);
            let _ = this.update(cx, |query, cx| query.settle(&key, generation, result, cx));
        }));
        cx.notify();
    }

    fn settle(&mut self, key: &K, generation: u64, result: Result<V, QueryError>, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get_mut(key) else { return };
        if let Err(e) = &result
            && entry.generation == generation
        {
            log::error!("Loading {} failed: {e:#}", (self.describe)(key));
        }
        if entry.settle(generation, result) {
            cx.notify();
        }
    }
}

/// One key's state. The transitions live here, apart from gpui, so they can be tested without an app.
struct Entry<V> {
    load: Load<V>,
    /// Failure of the latest fetch while `load` keeps an older value.
    refetch_error: Option<QueryError>,
    /// When `load` was filled by a fetch or [`Entry::put`], `None` once invalidated.
    fetched_at: Option<Instant>,
    /// Advanced by every fetch that starts and by every [`Entry::put`]: only the latest fetch may store its result.
    generation: u64,
    /// The running fetch.
    task: Option<Task<()>>,
    /// Callers waiting for the running fetch.
    waiters: Vec<Waiter<V>>,
}

impl<V> Default for Entry<V> {
    fn default() -> Self {
        Self { load: Load::Idle, refetch_error: None, fetched_at: None, generation: 0, task: None, waiters: Vec::new() }
    }
}

impl<V: Clone> Entry<V> {
    fn is_fresh(&self, stale: Duration) -> bool {
        matches!(self.load, Load::Ready(_))
            && self.refetch_error.is_none()
            && self.fetched_at.is_some_and(|at| at.elapsed() < stale)
    }

    fn wait(&mut self) -> Receiver<V> {
        let (tx, rx) = async_channel::bounded(1);
        self.waiters.push(tx);
        Receiver(rx)
    }

    /// Starts a fetch and returns its generation. A value already shown stays until the fetch settles.
    fn begin(&mut self) -> u64 {
        self.generation += 1;
        if !matches!(self.load, Load::Ready(_)) {
            self.load = Load::Pending;
        }
        self.generation
    }

    /// Stores the result of fetch `generation` and hands it to the waiting callers. Returns false, and changes nothing,
    /// for a fetch that a later one replaced.
    fn settle(&mut self, generation: u64, result: Result<V, QueryError>) -> bool {
        if generation != self.generation {
            return false;
        }
        // This runs inside the task itself, which must not cancel itself by dropping its handle.
        if let Some(task) = self.task.take() {
            task.detach();
        }
        match &result {
            Ok(value) => {
                self.load = Load::Ready(value.clone());
                self.refetch_error = None;
                self.fetched_at = Some(Instant::now());
            }
            Err(e) if matches!(self.load, Load::Ready(_)) => self.refetch_error = Some(e.clone()),
            Err(e) => self.load = Load::Failed(e.clone()),
        }
        for waiter in self.waiters.drain(..) {
            let _ = waiter.try_send(result.clone());
        }
        true
    }

    fn put(&mut self, value: V) {
        self.generation += 1;
        self.task = None;
        for waiter in self.waiters.drain(..) {
            let _ = waiter.try_send(Ok(value.clone()));
        }
        self.load = Load::Ready(value);
        self.refetch_error = None;
        self.fetched_at = Some(Instant::now());
    }
}

struct Receiver<V>(async_channel::Receiver<Result<V, QueryError>>);

impl<V: 'static> Receiver<V> {
    fn receive<T: 'static>(self, cx: &mut Context<T>) -> Task<Result<V, QueryError>> {
        cx.spawn(async move |_, _| {
            self.0.recv().await.unwrap_or_else(|_| Err(QueryError::from(anyhow!("the lookup was dropped"))))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(message: &str) -> QueryError {
        QueryError::from(anyhow!("{message}"))
    }

    fn received(rx: &Receiver<i32>) -> Option<Result<i32, String>> {
        rx.0.try_recv().ok().map(|r| r.map_err(|e| e.to_string()))
    }

    #[test]
    fn first_fetch_is_pending_then_ready_or_failed() {
        let mut e = Entry::<i32>::default();
        let g = e.begin();
        assert!(e.load.is_pending());
        assert!(e.settle(g, Ok(1)));
        assert_eq!(e.load.ready(), Some(&1));
        assert!(e.is_fresh(Duration::from_secs(60)));

        let mut e = Entry::<i32>::default();
        let g = e.begin();
        e.settle(g, Err(failure("offline")));
        assert!(matches!(&e.load, Load::Failed(err) if err.to_string() == "offline"));
        assert!(!e.is_fresh(Duration::MAX), "a failure is fetched again");
    }

    #[test]
    fn refetch_keeps_the_value_and_records_its_failure() {
        let mut e = Entry::<i32>::default();
        let g = e.begin();
        e.settle(g, Ok(1));
        let g = e.begin();
        assert_eq!(e.load.ready(), Some(&1), "the value stays visible while it is fetched again");
        e.settle(g, Err(failure("offline")));
        assert_eq!(e.load.ready(), Some(&1));
        assert_eq!(e.refetch_error.as_ref().map(ToString::to_string).as_deref(), Some("offline"));
        assert!(!e.is_fresh(Duration::MAX), "a failed refetch is retried");
        let g = e.begin();
        e.settle(g, Ok(2));
        assert!(e.refetch_error.is_none() && e.is_fresh(Duration::MAX));
    }

    #[test]
    fn a_replaced_fetch_cannot_store_its_result_but_its_callers_get_the_new_one() {
        let mut e = Entry::<i32>::default();
        let rx = e.wait();
        let old = e.begin();
        let new = e.begin();
        assert!(!e.settle(old, Ok(1)), "an older fetch must not overwrite a newer one");
        assert!(e.load.is_pending());
        assert_eq!(received(&rx), None);
        assert!(e.settle(new, Ok(2)));
        assert_eq!(e.load.ready(), Some(&2));
        assert_eq!(received(&rx), Some(Ok(2)));
    }

    #[test]
    fn every_waiter_of_a_fetch_gets_its_result_once() {
        let mut e = Entry::<i32>::default();
        let (a, b) = (e.wait(), e.wait());
        let g = e.begin();
        e.settle(g, Err(failure("boom")));
        assert_eq!(received(&a), Some(Err("boom".into())));
        assert_eq!(received(&b), Some(Err("boom".into())));
        let c = e.wait();
        let g = e.begin();
        e.settle(g, Ok(3));
        assert_eq!(received(&c), Some(Ok(3)));
        assert_eq!(received(&a), None);
    }

    #[test]
    fn put_answers_waiters_and_outranks_a_running_fetch() {
        let mut e = Entry::<i32>::default();
        let rx = e.wait();
        let running = e.begin();
        e.put(7);
        assert_eq!(received(&rx), Some(Ok(7)));
        assert!(!e.settle(running, Ok(1)));
        assert_eq!(e.load.ready(), Some(&7));
        assert!(e.is_fresh(Duration::from_secs(1)));
    }

    #[test]
    fn freshness_follows_the_stale_time() {
        let mut e = Entry::<i32>::default();
        let g = e.begin();
        e.settle(g, Ok(1));
        assert!(!e.is_fresh(Duration::ZERO), "a zero stale time fetches every time");
        e.fetched_at = None;
        assert!(!e.is_fresh(Duration::MAX), "an invalidated value is fetched again");
    }

    #[test]
    fn error_keeps_its_chain_and_types() {
        let inner = anyhow::Error::new(crate::backend::api::ApiError::NotFound).context("load release of x");
        let e = QueryError::from(inner);
        assert_eq!(e.to_string(), "load release of x");
        assert_eq!(format!("{e:#}"), "load release of x: not found");
        assert_eq!(e.message(), "Load release of x: not found");
        let back = anyhow::Error::new(e);
        assert_eq!(format!("{back:#}"), "load release of x: not found");
        assert!(back.chain().any(|c| matches!(c.downcast_ref(), Some(crate::backend::api::ApiError::NotFound))));
    }

    #[test]
    fn operations_that_failed_on_a_query_are_recognised_through_context() {
        let from_query = anyhow::Error::new(failure("offline"));
        assert!(is_query_failure(&from_query));
        assert!(is_query_failure(&from_query.context("install x")));
        assert!(!is_query_failure(&anyhow!("disk full").context("install x")));
    }
}
