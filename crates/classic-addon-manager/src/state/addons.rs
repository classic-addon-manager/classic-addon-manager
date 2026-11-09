//! Installed addons, update state, and the installs, updates and uninstalls that run on them.
//!
//! Use the entity from a view: `let store = state::addons(cx);` then `cx.observe(&store, |this, _, cx| cx.notify())`
//! and `store.read(cx).installed_addons`. While an action changes an addon, [`AddonState::operation`] holds its
//! progress, and every other action on that addon is rejected. An install holds the dependencies it installs too.
//! Actions return a task that resolves once the change is done and the installed list is reloaded. Await it outside
//! the store's update, so the caller may update `AddonState` again:
//! ```ignore
//! let uninstall = state::addons(cx).update(cx, |s, cx| s.uninstall(name, cx));
//! cx.spawn(async move |cx| match uninstall.await { /* anyhow::Result<()> */ }).detach();
//! ```
//! A failed action resolves to its error unlogged: the caller that shows the error logs it. An action runs to its end
//! even when nobody awaits its task, so its operation always ends.
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use cam_core::{
    Backend,
    addon::{
        self, Addon, AddonDir, DependencyResolutionResult, InstallError, InstallProgress, InstallWithDependenciesResult,
    },
    api::{self, AddonManifest, Release},
};
use gpui_kit::{AsyncApp, Context, Task, WeakEntity};

use super::{
    backend,
    catalog::{self, Catalog},
    log_failure,
    tasks::{hold_spinner, run_blocking},
};

fn count_updates(managed: &[Addon], releases: &HashMap<String, Release>) -> usize {
    managed.iter().filter(|a| releases.get(&a.name).is_some_and(|r| addon::has_update(a, r))).count()
}

/// An install, update or uninstall running on one addon.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Operation {
    /// The catalog arrived and the manifest was found, so the download comes next. Stays false for uninstalls, ZIP
    /// installs and the dependencies an install holds.
    pub manifest_resolved: bool,
    /// Latest download or extraction event. Missing dependencies report first, then the addon itself.
    pub progress: Option<InstallProgress>,
}

/// Outcome of [`AddonState::restore_subscribed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Restored {
    pub installed: usize,
    pub failed: usize,
}

pub struct AddonState {
    pub installed_addons: Vec<Addon>,
    pub updates_available_count: usize,
    pub is_checking_for_updates: bool,
    /// name -> latest release (from the bulk update check)
    pub latest_releases: HashMap<String, Release>,
    update_check_queued: bool,
    /// The installs, updates and uninstalls running, keyed by addon name.
    operations: HashMap<String, Operation>,
    /// ZIP archives installing or waiting to.
    zip_installs: usize,
    /// Closes once the ZIP archive given last has finished. The archive given next waits for that.
    zip_queue: Option<async_channel::Receiver<()>>,
    restoring: bool,
}

impl AddonState {
    pub fn new() -> Self {
        Self {
            installed_addons: Vec::new(),
            updates_available_count: 0,
            is_checking_for_updates: false,
            latest_releases: HashMap::new(),
            update_check_queued: false,
            operations: HashMap::new(),
            zip_installs: 0,
            zip_queue: None,
            restoring: false,
        }
    }

    pub fn is_installed(&self, name: &str) -> bool {
        self.installed_addons.iter().any(|a| a.name == name)
    }

    /// The install, update or uninstall running on `name`.
    pub fn operation(&self, name: &str) -> Option<&Operation> {
        self.operations.get(name)
    }

    /// True while a ZIP archive is installing or waiting to.
    pub fn is_installing_zip(&self) -> bool {
        self.zip_installs > 0
    }

    /// Reloads the installed addons from disk. Resolves once the list is replaced.
    pub fn update_installed_addons(&mut self, cx: &mut Context<Self>) -> Task<()> {
        let backend = backend(cx);
        cx.spawn(async move |this, cx| {
            let addons = run_blocking(cx, move || installed_addons(&backend)).await;
            let _ = this.update(cx, |this, cx| {
                this.installed_addons = addons;
                cx.notify();
            });
        })
    }

    /// Reloads the installed addons from disk, then checks them for updates. Resolves once the list is replaced, the
    /// update check goes on in the background.
    pub fn refresh(&mut self, cx: &mut Context<Self>) -> Task<()> {
        let reload = self.update_installed_addons(cx);
        cx.spawn(async move |this, cx| {
            reload.await;
            let _ = this.update(cx, |this, cx| this.perform_bulk_update_check(cx));
        })
    }

    /// Queries the latest release of every managed addon.
    pub fn perform_bulk_update_check(&mut self, cx: &mut Context<Self>) {
        if self.is_checking_for_updates {
            self.update_check_queued = true;
            return;
        }
        self.is_checking_for_updates = true;
        cx.notify();
        let started = Instant::now();
        let managed: Vec<Addon> = self.installed_addons.iter().filter(|a| a.is_managed).cloned().collect();
        let names: Vec<String> = managed.iter().map(|a| a.name.clone()).collect();

        if names.is_empty() {
            self.latest_releases.clear();
            self.updates_available_count = 0;
            self.finish_update_check(started, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            let res = run_blocking(cx, move || api::latest_releases(&names)).await;
            this.update(cx, |this, cx| {
                match res {
                    Err(e) => {
                        log::error!("Bulk update check failed: {e:#}");
                        // keep the last good releases so badges survive a network hiccup, but recount
                        this.updates_available_count = count_updates(&managed, &this.latest_releases);
                    }
                    Ok(releases) => {
                        let count = count_updates(&managed, &releases);
                        if !this.update_check_queued {
                            this.latest_releases = releases;
                            this.updates_available_count = count;
                        }
                    }
                }
                this.finish_update_check(started, cx);
            })
        })
        .detach();
    }

    fn finish_update_check(&mut self, started: Instant, cx: &mut Context<Self>) {
        cx.notify();
        cx.spawn(async move |this, cx| {
            hold_spinner(started, cx).await;
            let _ = this.update(cx, |this, cx| {
                this.is_checking_for_updates = false;
                cx.notify();
                if this.update_check_queued {
                    this.update_check_queued = false;
                    this.perform_bulk_update_check(cx);
                }
            });
        })
        .detach();
    }

    /// Installs `manifest` at `version` with its missing dependencies.
    pub fn install(
        &mut self,
        manifest: AddonManifest,
        version: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<InstallWithDependenciesResult>> {
        self.run_change(
            manifest.name.clone(),
            cx,
            move |_| Ok(manifest),
            move |dir, manifest, resolution, progress| {
                dir.install_addon_with_dependencies(manifest, &version, resolution, progress)
            },
        )
    }

    /// Updates the installed addon `name` to `version`, installing missing dependencies first. The manifest comes from
    /// the catalog.
    pub fn update(
        &mut self,
        name: String,
        version: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<InstallWithDependenciesResult>> {
        let lookup = name.clone();
        let find = move |catalog: &Catalog| find_manifest(catalog, &lookup);
        self.run_change(name, cx, find, move |dir, manifest, resolution, progress| {
            dir.update_addon_with_dependencies(manifest, &version, resolution, progress)
        })
    }

    /// Gets the catalog, which dependencies are resolved against, and the manifest from `manifest`. The missing
    /// dependencies are busy as well while `work` installs them. Runs `work` in the background with its progress going
    /// into the operations, and refreshes the store once it returned.
    fn run_change(
        &mut self,
        name: String,
        cx: &mut Context<Self>,
        manifest: impl FnOnce(&Catalog) -> Result<AddonManifest> + 'static,
        work: impl FnOnce(
            &AddonDir,
            &AddonManifest,
            DependencyResolutionResult,
            &dyn Fn(&InstallProgress),
        ) -> InstallWithDependenciesResult
        + Send
        + 'static,
    ) -> Task<Result<InstallWithDependenciesResult>> {
        let backend = backend(cx);
        self.run(vec![name.clone()], cx, async move |this, cx| {
            let catalog = cx.update(catalog::fetch).await?;
            let manifest = manifest(&catalog)?;
            let resolution = {
                let (manifest, backend) = (manifest.clone(), backend.clone());
                run_blocking(cx, move || backend.addon_dir()?.resolve_dependencies(&manifest, catalog.addons())).await?
            };
            let missing: Vec<String> =
                resolution.dependencies.iter().filter(|d| !d.is_installed).map(|d| d.manifest.name.clone()).collect();
            this.update(cx, |s, cx| {
                s.begin(&missing)?;
                s.track(&name, |op| op.manifest_resolved = true, cx);
                anyhow::Ok(())
            })??;
            let result = run_work(&this, name, cx, move |progress| {
                Ok(work(&backend.addon_dir()?, &manifest, resolution, progress))
            })
            .await;
            let _ = this.update(cx, |s, cx| {
                s.end(&missing);
                cx.notify();
            });
            result
        })
    }

    /// Uninstalls the addon `name`.
    pub fn uninstall(&mut self, name: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let backend = backend(cx);
        self.run(vec![name.clone()], cx, async move |this, cx| {
            let result = run_blocking(cx, move || backend.addon_dir()?.uninstall_addon(&name)).await;
            refresh(&this, cx).await;
            result
        })
    }

    /// Uninstalls every installed addon, one after another, and reloads the installed list once at the end. Stops at
    /// the first failure, whose error names the addon. Resolves to the number of addons removed.
    pub fn uninstall_all(&mut self, cx: &mut Context<Self>) -> Task<Result<usize>> {
        let addons: Vec<(String, String)> =
            self.installed_addons.iter().map(|a| (a.name.clone(), a.alias.clone())).collect();
        let names = addons.iter().map(|(name, _)| name.clone()).collect();
        let backend = backend(cx);
        self.run(names, cx, async move |this, cx| {
            let mut result = Ok(addons.len());
            for (name, alias) in addons {
                let backend = backend.clone();
                if let Err(e) = run_blocking(cx, move || backend.addon_dir()?.uninstall_addon(&name)).await {
                    result = Err(e.context(format!("failed to uninstall addon {alias}")));
                    break;
                }
            }
            refresh(&this, cx).await;
            result
        })
    }

    /// Stops managing the addon `name`: its files stay, updates are no longer tracked.
    pub fn unmanage(&mut self, name: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let backend = backend(cx);
        self.run(vec![name.clone()], cx, async move |this, cx| {
            let result = run_blocking(cx, move || backend.addon_dir()?.remove_managed_addon(&name)).await;
            refresh(&this, cx).await;
            result
        })
    }

    /// Installs the addon in the ZIP archive at `path`, then reloads the installed list. Archives install one after
    /// another in the order they were given, so this one may wait for earlier ones. Resolves to the addon's name.
    pub fn install_zip(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Task<Result<String>> {
        // Nothing is ever sent on `done`. Dropping it closes the channel, which lets the next archive start.
        let (done, finished) = async_channel::bounded::<()>(1);
        let previous = self.zip_queue.replace(finished);
        self.zip_installs += 1;
        cx.notify();
        let backend = backend(cx);
        run_to_end(cx, async move |this, cx| {
            if let Some(previous) = previous {
                let _ = previous.recv().await;
            }
            let result = install_archive(&this, &backend, path, cx).await;
            let _ = this.update(cx, |s, cx| {
                s.zip_installs -= 1;
                cx.notify();
            });
            // Lets the next archive start. Naming `done` here is what moves it into this task.
            drop(done);
            result
        })
    }

    /// Installs every subscribed addon that is missing locally, one after another. Resolves to how many were restored
    /// and how many failed, both zero when a restore runs already. Each failure is logged.
    pub fn restore_subscribed(&mut self, cx: &mut Context<Self>) -> Task<Result<Restored>> {
        if self.restoring {
            return Task::ready(Ok(Restored::default()));
        }
        self.restoring = true;
        let backend = backend(cx);
        run_to_end(cx, async move |this, cx| {
            let result = restore(&this, &backend, cx).await;
            let _ = this.update(cx, |s, _| s.restoring = false);
            result
        })
    }

    /// Marks `names` busy, runs `work` to its end and clears them once it returned. Rejects the call when one of
    /// `names` is busy already.
    fn run<T: 'static>(
        &mut self,
        names: Vec<String>,
        cx: &mut Context<Self>,
        work: impl AsyncFnOnce(WeakEntity<Self>, &mut AsyncApp) -> Result<T> + 'static,
    ) -> Task<Result<T>> {
        if let Err(e) = self.begin(&names) {
            return Task::ready(Err(e));
        }
        cx.notify();
        run_to_end(cx, async move |this, cx| {
            let result = work(this.clone(), cx).await;
            let _ = this.update(cx, |s, cx| {
                s.end(&names);
                cx.notify();
            });
            result
        })
    }

    /// Starts an operation on every addon of `names`, or on none of them when one is busy already. Names are compared
    /// without case, because a case-insensitive file system has one folder for both spellings.
    fn begin(&mut self, names: &[String]) -> Result<()> {
        let busy = |name: &&String| self.operations.keys().any(|running| running.eq_ignore_ascii_case(name));
        if let Some(busy) = names.iter().find(busy) {
            bail!("{busy} is busy: another install, update or uninstall of it is running");
        }
        for name in names {
            self.operations.insert(name.clone(), Operation::default());
        }
        Ok(())
    }

    fn end(&mut self, names: &[String]) {
        for name in names {
            self.operations.remove(name);
        }
    }

    /// Applies `change` to the operation on `name`. Does nothing once it ended.
    fn track(&mut self, name: &str, change: impl FnOnce(&mut Operation), cx: &mut Context<Self>) {
        if let Some(op) = self.operations.get_mut(name) {
            change(op);
            cx.notify();
        }
    }
}

/// The catalog entry of `name`. An addon missing from the catalog has nothing to install: [`InstallError::NoRelease`].
fn find_manifest(catalog: &Catalog, name: &str) -> Result<AddonManifest> {
    catalog.manifest(name).cloned().ok_or_else(|| {
        anyhow::Error::new(InstallError::NoRelease).context(format!("addon {name} is not in the catalog"))
    })
}

/// The installed addons, none when addons.txt can't be read. The failure is logged.
fn installed_addons(backend: &Backend) -> Vec<Addon> {
    match backend.addon_dir().and_then(|dir| dir.addons()) {
        Ok(addons) => addons,
        Err(e) => {
            log::error!("Reading the installed addons failed: {e:#}");
            Vec::new()
        }
    }
}

/// The addons addons.txt lists now. A missing addons.txt is created, as an install would.
async fn listed_addons(backend: &Arc<Backend>, cx: &mut AsyncApp) -> Result<Vec<String>> {
    let backend = backend.clone();
    run_blocking(cx, move || {
        let addon_dir = backend.addon_dir()?;
        addon_dir.ensure_addons_txt_exists()?;
        addon_dir.installed_names()
    })
    .await
}

/// Runs `work` in a task that is never cancelled and returns a task with its result. Dropping the returned task only
/// drops the result.
fn run_to_end<T: 'static>(
    cx: &mut Context<AddonState>,
    work: impl AsyncFnOnce(WeakEntity<AddonState>, &mut AsyncApp) -> Result<T> + 'static,
) -> Task<Result<T>> {
    let (tx, rx) = async_channel::bounded(1);
    cx.spawn(async move |this, cx| {
        let _ = tx.try_send(work(this, cx).await);
    })
    .detach();
    cx.spawn(async move |_, _| rx.recv().await.unwrap_or_else(|_| Err(anyhow!("the app stopped the operation"))))
}

/// Runs `work` in the background while its progress goes into the operation on `name`, and into the operation of the
/// dependency it reports on. Returns once every event is applied and the store refreshed.
async fn run_work(
    this: &WeakEntity<AddonState>,
    name: String,
    cx: &mut AsyncApp,
    work: impl FnOnce(&dyn Fn(&InstallProgress)) -> Result<InstallWithDependenciesResult> + Send + 'static,
) -> Result<InstallWithDependenciesResult> {
    let (tx, rx) = async_channel::unbounded::<InstallProgress>();
    let owner = this.clone();
    let forwarding = cx.spawn(async move |cx| {
        // Stops once the work drops the sender.
        while let Ok(event) = rx.recv().await {
            let _ = owner.update(cx, |s, cx| {
                if event.name != name {
                    s.track(&event.name, |op| op.progress = Some(event.clone()), cx);
                }
                s.track(&name, |op| op.progress = Some(event), cx);
            });
        }
    });
    let result = run_blocking(cx, move || {
        work(&|event: &InstallProgress| {
            let _ = tx.try_send(event.clone());
        })
    })
    .await;
    // Every event lands before the operations end, so none can reach a later operation on the same addon.
    forwarding.await;
    refresh(this, cx).await;
    result
}

/// [`AddonState::install_zip`] once the archive is first in line. The addon it installs as is busy meanwhile.
async fn install_archive(
    this: &WeakEntity<AddonState>,
    backend: &Arc<Backend>,
    path: PathBuf,
    cx: &mut AsyncApp,
) -> Result<String> {
    let name = {
        let (path, backend) = (path.clone(), backend.clone());
        run_blocking(cx, move || backend.addon_dir()?.zip_addon_name(&path)).await?
    };
    let names = vec![name.clone()];
    this.update(cx, |s, cx| {
        s.begin(&names)?;
        cx.notify();
        anyhow::Ok(())
    })??;
    let (target, backend) = (name.clone(), backend.clone());
    let result = run_blocking(cx, move || backend.addon_dir()?.install_zip(&path, &target)).await;
    if result.is_ok()
        && let Ok(reload) = this.update(cx, |s, cx| s.update_installed_addons(cx))
    {
        reload.await;
    }
    let _ = this.update(cx, |s, cx| {
        s.end(&names);
        cx.notify();
    });
    result.map(|()| name)
}

/// [`AddonState::restore_subscribed`] without the guard against a second restore.
async fn restore(this: &WeakEntity<AddonState>, backend: &Arc<Backend>, cx: &mut AsyncApp) -> Result<Restored> {
    let subscribed = run_blocking(cx, api::subscribed_addons).await.context("fetching subscribed addons")?;
    let listed = listed_addons(backend, cx).await.context("reading the installed addons")?;
    let missing: Vec<AddonManifest> = subscribed.into_iter().filter(|a| !listed.contains(&a.name)).collect();
    let mut restored = Restored::default();
    if missing.is_empty() {
        return Ok(restored);
    }
    // Checked once up front, so an unreachable catalog costs one warning, not one failure per addon. The catalog logs
    // why it failed.
    if cx.update(catalog::fetch).await.is_err() {
        restored.failed = missing.len();
        log::warn!("Restoring {} subscribed addon(s) skipped: the catalog is unavailable", missing.len());
        return Ok(restored);
    }
    for addon in missing {
        // An addon restored earlier in this loop, as a dependency of another one, is there already. An addon that is
        // installing elsewhere is left to that install.
        let name = addon.name.clone();
        let installing = this.read_with(cx, |s, _| s.operation(&name).is_some())?;
        if installing || listed_addons(backend, cx).await.context("reading the installed addons")?.contains(&name) {
            continue;
        }
        let install = this.update(cx, |s, cx| s.install(addon, "latest".to_string(), cx))?;
        match install.await {
            Ok(result) if result.success => restored.installed += 1,
            // The install logged why it failed.
            Ok(_) => restored.failed += 1,
            Err(e) => {
                log_failure(&format!("Restoring {name} failed:"), &e);
                restored.failed += 1;
            }
        }
    }
    Ok(restored)
}

/// [`AddonState::refresh`] from inside one of the store's own tasks.
async fn refresh(this: &WeakEntity<AddonState>, cx: &mut AsyncApp) {
    if let Ok(refresh) = this.update(cx, |this, cx| this.refresh(cx)) {
        refresh.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn an_addon_runs_one_operation_at_a_time() {
        let mut store = AddonState::new();
        store.begin(&names(&["a"])).unwrap();
        assert!(store.begin(&names(&["a"])).is_err());
        assert!(store.begin(&names(&["A"])).is_err(), "one folder on a case-insensitive file system");
        assert!(store.begin(&names(&["b", "a"])).is_err());
        assert!(store.operation("b").is_none(), "a rejected batch starts none of its operations");
        store.end(&names(&["a"]));
        assert!(store.begin(&names(&["b", "a"])).is_ok());
        assert!(store.operation("a").is_some() && store.operation("b").is_some());
    }

    #[test]
    fn manifest_lookup() {
        let m = AddonManifest { name: "a".into(), ..Default::default() };
        let catalog = Catalog::new(vec![m]);
        assert!(find_manifest(&catalog, "a").is_ok());
        let missing = find_manifest(&catalog, "b").unwrap_err();
        assert_eq!(missing.to_string(), "addon b is not in the catalog");
        assert_eq!(InstallError::from_error(&missing), InstallError::NoRelease);
    }
}
