//! Dependencies of an addon: resolving them against the catalog, and installing the missing ones before the addon
//! itself. The catalog is the caller's, passed in as `catalog`.
use std::collections::{HashMap, HashSet};

use anyhow::Result;

use super::{
    AddonDir, AddonInstallStatus, DependencyInfo, DependencyResolutionResult, InstallError, InstallProgress,
    InstallWithDependenciesResult,
};
use crate::api::AddonManifest;

const MAX_DEPENDENCY_DEPTH: i32 = 10;

impl AddonDir<'_> {
    /// Dependency tree of `ad` against `catalog` and the addons installed now (read from addons.txt). A missing
    /// addons.txt is created, as the install that follows would.
    pub fn resolve_dependencies(
        &self,
        ad: &AddonManifest,
        catalog: &[AddonManifest],
    ) -> Result<DependencyResolutionResult> {
        self.ensure_addons_txt_exists()?;
        let installed: HashSet<String> = self.installed_names()?.into_iter().collect();
        Ok(resolve_dependencies_with(ad, catalog, &installed))
    }

    /// Installs the missing dependencies in `resolution` (see [`Self::resolve_dependencies`]), then `ad` at `version`.
    /// `progress` reports each download and extraction.
    pub fn install_addon_with_dependencies(
        &self,
        ad: &AddonManifest,
        version: &str,
        resolution: DependencyResolutionResult,
        progress: &dyn Fn(&InstallProgress),
    ) -> InstallWithDependenciesResult {
        apply_dependencies_then_parent(
            ad,
            resolution,
            &|dep, v| self.install_addon(dep, v, progress),
            &|| self.install_addon(ad, version, progress),
            &|| self.sort_addons_txt(),
        )
    }

    /// Installs the missing dependencies in `resolution` (see [`Self::resolve_dependencies`]), then updates `ad` to
    /// `version`. `progress` reports each download and extraction.
    pub fn update_addon_with_dependencies(
        &self,
        ad: &AddonManifest,
        version: &str,
        resolution: DependencyResolutionResult,
        progress: &dyn Fn(&InstallProgress),
    ) -> InstallWithDependenciesResult {
        apply_dependencies_then_parent(
            ad,
            resolution,
            &|dep, v| self.install_addon(dep, v, progress),
            &|| self.update_addon(ad, version, progress),
            &|| self.sort_addons_txt(),
        )
    }
}

pub fn resolve_dependencies_with(
    ad: &AddonManifest,
    manifests: &[AddonManifest],
    installed: &HashSet<String>,
) -> DependencyResolutionResult {
    struct Walker<'a> {
        by_name: HashMap<&'a str, &'a AddonManifest>,
        installed: &'a HashSet<String>,
        deps: HashMap<String, DependencyInfo>,
        errors: Vec<String>,
        path: HashSet<String>,
    }
    impl Walker<'_> {
        fn walk(&mut self, name: &str, depth: i32) {
            if depth > MAX_DEPENDENCY_DEPTH {
                self.errors.push(format!("Maximum dependency depth exceeded for: {name}"));
                return;
            }
            if self.path.contains(name) {
                self.errors.push(format!("Circular dependency detected: {name}"));
                return;
            }
            let Some(&manifest) = self.by_name.get(name) else {
                self.errors.push(format!("Addon {name} not found in repository manifests"));
                return;
            };
            let is_installed = self.installed.contains(name);
            let replace = match self.deps.get(name) {
                None => true,
                Some(existing) => depth > existing.depth,
            };
            if replace {
                self.deps.insert(name.to_string(), DependencyInfo { manifest: manifest.clone(), is_installed, depth });
            }
            self.path.insert(name.to_string());
            for dep in &manifest.dependencies {
                self.walk(dep, depth + 1);
            }
            self.path.remove(name);
        }
    }

    let mut w = Walker {
        by_name: manifests.iter().map(|m| (m.name.as_str(), m)).collect(),
        installed,
        deps: HashMap::new(),
        errors: Vec::new(),
        path: HashSet::from([ad.name.clone()]),
    };
    for dep in &ad.dependencies {
        w.walk(dep, 0);
    }

    let mut dependencies: Vec<DependencyInfo> = w.deps.into_values().collect();
    dependencies.sort_by(|a, b| b.depth.cmp(&a.depth).then_with(|| a.manifest.name.cmp(&b.manifest.name)));

    DependencyResolutionResult { dependencies, errors: w.errors }
}

/// Install/parent/sort failures are recorded on the returned result, and logged here because this is where they
/// stop propagating. Dependency resolution errors come back as the result's dependency warnings.
///
/// Dependencies that are already installed are left alone, even when a newer version exists: installing one
/// addon should not silently change another one, outdated addons go through the normal update flow.
fn apply_dependencies_then_parent(
    ad: &AddonManifest,
    resolution: DependencyResolutionResult,
    install_dep: &dyn Fn(&AddonManifest, &str) -> Result<()>,
    parent: &dyn Fn() -> Result<()>,
    sort_addons: &dyn Fn() -> Result<()>,
) -> InstallWithDependenciesResult {
    let mut result = InstallWithDependenciesResult {
        success: false,
        dependency_warnings: resolution.errors,
        dependencies: Vec::new(),
        main_addon: AddonInstallStatus { name: ad.name.clone(), alias: ad.alias.clone(), ..Default::default() },
    };

    for dep in &resolution.dependencies {
        let mut status = AddonInstallStatus {
            name: dep.manifest.name.clone(),
            alias: dep.manifest.alias.clone(),
            ..Default::default()
        };
        if status.alias.is_empty() {
            status.alias = dep.manifest.name.clone();
        }

        if dep.is_installed {
            status.success = true;
            status.skipped = true;
            result.dependencies.push(status);
            continue;
        }

        if let Err(e) = install_dep(&dep.manifest, "latest") {
            log::error!("Installing dependency {} of {} failed: {e:#}", dep.manifest.name, ad.name);
            status.error = Some(InstallError::from_error(&e));
            result.dependencies.push(status);
            result.main_addon.error = Some(InstallError::Dependency);
            return result;
        }
        status.success = true;
        result.dependencies.push(status);
    }

    if let Err(e) = parent() {
        log::error!("Installing {} failed: {e:#}", ad.name);
        result.main_addon.error = Some(InstallError::from_error(&e));
        return result;
    }

    result.main_addon.success = true;
    result.success = true;

    if let Err(e) = sort_addons() {
        log::error!("Sorting addons.txt after an install failed: {e:#}");
        result.dependency_warnings.push(format!("{e:#}"));
        result.main_addon.error = Some(InstallError::Failed(format!("{e:#}")));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use anyhow::anyhow;
    use std::cell::RefCell;

    fn m(name: &str, deps: &[&str]) -> AddonManifest {
        AddonManifest {
            name: name.into(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn resolves_transitive_dependencies_deepest_first() {
        let manifests = vec![m("top", &["a"]), m("a", &["b"]), m("b", &["c"]), m("c", &[])];
        let r = resolve_dependencies_with(&manifests[0], &manifests, &HashSet::from(["b".to_string()]));
        let names: Vec<_> =
            r.dependencies.iter().map(|d| (d.manifest.name.as_str(), d.depth, d.is_installed)).collect();
        assert_eq!(names, vec![("c", 2, false), ("b", 1, true), ("a", 0, false)]);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn reports_missing_and_circular() {
        let manifests = vec![m("top", &["a", "ghost"]), m("a", &["top"])];
        let r = resolve_dependencies_with(&manifests[0], &manifests, &HashSet::new());
        assert_eq!(
            r.errors,
            vec![
                "Circular dependency detected: top".to_string(),
                "Addon ghost not found in repository manifests".to_string()
            ]
        );
        assert_eq!(r.dependencies.len(), 1);
    }

    #[test]
    fn shared_dependency_keeps_deepest_depth() {
        let manifests = vec![m("top", &["a", "b"]), m("a", &["b"]), m("b", &[])];
        let r = resolve_dependencies_with(&manifests[0], &manifests, &HashSet::new());
        assert_eq!(r.dependencies[0].manifest.name, "b");
        assert_eq!(r.dependencies[0].depth, 1);
    }

    #[test]
    fn depth_limit() {
        let mut manifests = vec![m("top", &["d0"])];
        for i in 0..13 {
            manifests.push(m(&format!("d{i}"), &[&format!("d{}", i + 1)]));
        }
        manifests.push(m("d13", &[]));
        let r = resolve_dependencies_with(&manifests[0], &manifests, &HashSet::new());
        assert_eq!(r.errors, vec!["Maximum dependency depth exceeded for: d11".to_string()]);
    }

    fn resolution(deps: Vec<(&str, bool)>) -> DependencyResolutionResult {
        DependencyResolutionResult {
            dependencies: deps
                .into_iter()
                .map(|(n, i)| DependencyInfo { manifest: m(n, &[]), is_installed: i, depth: 0 })
                .collect(),
            errors: vec!["warn".into()],
        }
    }

    #[test]
    fn installs_missing_dependencies_then_parent() {
        let installed = RefCell::new(Vec::new());
        let ad = m("main", &[]);
        let r = apply_dependencies_then_parent(
            &ad,
            resolution(vec![("a", false), ("b", true)]),
            &|d, v| {
                installed.borrow_mut().push(format!("{}@{v}", d.name));
                Ok(())
            },
            &|| Ok(()),
            &|| Ok(()),
        );
        assert!(r.success && r.main_addon.success);
        assert_eq!(*installed.borrow(), vec!["a@latest".to_string()]);
        assert!(r.dependencies[1].skipped && r.dependencies[1].success);
        assert_eq!(r.dependency_warnings, vec!["warn".to_string()]);
    }

    #[test]
    fn dependency_failure_stops_before_parent() {
        let ad = m("main", &[]);
        let r = apply_dependencies_then_parent(
            &ad,
            resolution(vec![("a", false), ("b", false)]),
            &|_, _| Err(anyhow!("nope")),
            &|| panic!("parent must not run"),
            &|| Ok(()),
        );
        assert!(!r.success);
        assert_eq!(r.dependencies.len(), 1);
        assert_eq!(r.dependencies[0].error, Some(InstallError::Failed("nope".into())));
        assert_eq!(r.main_addon.error, Some(InstallError::Dependency));
    }

    #[test]
    fn parent_error_and_sort_error() {
        let ad = m("main", &[]);
        let r =
            apply_dependencies_then_parent(&ad, resolution(vec![]), &|_, _| Ok(()), &|| Err(anyhow!("boom")), &|| {
                Ok(())
            });
        assert!(!r.success && !r.main_addon.success);
        assert_eq!(r.main_addon.error, Some(InstallError::Failed("boom".into())));
        let r = apply_dependencies_then_parent(&ad, resolution(vec![]), &|_, _| Ok(()), &|| Ok(()), &|| {
            Err(anyhow!("cycle"))
        });
        assert!(r.success);
        assert_eq!(r.main_addon.error, Some(InstallError::Failed("cycle".into())));
        assert_eq!(r.dependency_warnings.last().unwrap(), "cycle");
    }

    #[test]
    fn failures_keep_their_kind_through_added_context() {
        let parent_failing_with = |e: anyhow::Error| {
            let e = std::cell::RefCell::new(Some(e.context("installing main")));
            apply_dependencies_then_parent(
                &m("main", &[]),
                resolution(vec![]),
                &|_, _| Ok(()),
                &|| Err(e.take().unwrap()),
                &|| Ok(()),
            )
            .main_addon
            .error
        };
        let mismatch = crate::download::VerificationFailed("SHA-256 mismatch".into());
        assert_eq!(parent_failing_with(mismatch.into()), Some(InstallError::Verification("SHA-256 mismatch".into())));
        assert_eq!(parent_failing_with(api::ApiError::NotFound.into()), Some(InstallError::NoRelease));
        let other = api::ApiError::Rejected { status: 500, message: "failed to get release".into() };
        assert_eq!(
            parent_failing_with(other.into()),
            Some(InstallError::Failed("installing main: failed to get release (HTTP 500)".into()))
        );
    }
}
