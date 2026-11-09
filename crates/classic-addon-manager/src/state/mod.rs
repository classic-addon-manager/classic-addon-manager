//! Global application state, held in GPUI entities.
//!
//! * `state::user(cx)`, `settings(cx)`, `preferences(cx)`, `addons(cx)`, `app_info(cx)` return `Entity<..>` handles.
//!   Read them with `.read(cx)`, change them with `.update(cx, |s, cx| ..)` and observe them with
//!   `cx.observe(&entity, ..)`.
//! * `state::backend(cx)` is the [`Backend`]: the app's folders, the settings and the addon folder. Clone it into the
//!   work given to `run_blocking`.
//! * `state::run_blocking(cx, work)` runs a blocking `cam_core` call off the UI thread and returns its task (see
//!   `tasks`). Store actions return tasks as well: await them inside `cx.spawn` for their result.
//! * `state::catalog(cx)` is the addon catalog (see `catalog`), `state::queries::*` the cached addon details, both
//!   built on [`Query`] and [`Load`] (see `query`).
use std::sync::Arc;

use cam_core::Backend;
use gpui_kit::{App, AppContext, Entity, Global};

pub mod addons;
pub mod app;
pub mod catalog;
pub mod preferences;
pub mod queries;
pub mod query;
pub mod ratings;
pub mod settings;
pub mod tasks;
pub mod user;

pub use addons::{AddonState, Operation};
pub use app::AppInfoState;
pub use preferences::PreferencesState;
pub use query::{Load, Query, QueryError, log_failure};
pub use settings::SettingsState;
pub use tasks::{hold_spinner, run_blocking};
pub use user::UserState;

/// The store entities, held in one gpui global. Reach them through the functions below.
struct AppState {
    pub backend: Arc<Backend>,
    pub user: Entity<UserState>,
    pub settings: Entity<SettingsState>,
    pub preferences: Entity<PreferencesState>,
    pub addons: Entity<AddonState>,
    pub app_info: Entity<AppInfoState>,
    pub catalog: Entity<catalog::CatalogQuery>,
    queries: queries::Queries,
}

impl Global for AppState {}

/// Creates every store entity. Call once in `Application::run`, before opening windows.
pub fn init(cx: &mut App, backend: Arc<Backend>) {
    let state = AppState {
        backend,
        user: cx.new(|_| UserState::new()),
        settings: cx.new(|_| SettingsState::new()),
        preferences: cx.new(|_| PreferencesState::new()),
        addons: cx.new(|_| AddonState::new()),
        app_info: cx.new(|_| AppInfoState::new()),
        catalog: cx.new(|_| catalog::query()),
        queries: queries::Queries::new(cx),
    };
    cx.set_global(state);
}

/// Startup sequence: restores the stored session, hydrates preferences and loads the config.
pub fn bootstrap(cx: &mut App) {
    let s = cx.global::<AppState>();
    let (user, prefs, settings) = (s.user.clone(), s.preferences.clone(), s.settings.clone());
    user.update(cx, |u, cx| u.bootstrap_auth(cx));
    prefs.update(cx, |p, cx| p.hydrate(cx));
    settings.update(cx, |s, cx| s.load_config(cx));
}

/// True once auth bootstrap and preference hydration finished (the shell renders nothing before).
pub fn is_ready(cx: &App) -> bool {
    let s = cx.global::<AppState>();
    s.user.read(cx).auth_bootstrap_complete && s.preferences.read(cx).hydrated
}

pub fn backend(cx: &App) -> Arc<Backend> {
    cx.global::<AppState>().backend.clone()
}
pub fn user(cx: &App) -> Entity<UserState> {
    cx.global::<AppState>().user.clone()
}
pub fn settings(cx: &App) -> Entity<SettingsState> {
    cx.global::<AppState>().settings.clone()
}
pub fn preferences(cx: &App) -> Entity<PreferencesState> {
    cx.global::<AppState>().preferences.clone()
}
pub fn addons(cx: &App) -> Entity<AddonState> {
    cx.global::<AppState>().addons.clone()
}
pub fn app_info(cx: &App) -> Entity<AppInfoState> {
    cx.global::<AppState>().app_info.clone()
}
pub fn catalog(cx: &App) -> Entity<catalog::CatalogQuery> {
    cx.global::<AppState>().catalog.clone()
}
