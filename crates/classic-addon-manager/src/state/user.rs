//! Auth token, the signed-in `/me` user, sign in via deeplink token, sign out, subscription restore.
use anyhow::{Result, anyhow};
use cam_core::{
    api::{self, ApiError, User},
    auth,
};
use cam_ui::toast;
use gpui_kit::{App, Context, Task};

use super::{addons::Restored, backend, queries, tasks::run_blocking};

pub struct UserState {
    pub user: User,
    pub token: String,
    pub auth_bootstrap_complete: bool,
}

impl UserState {
    pub fn new() -> Self {
        Self { user: User::default(), token: String::new(), auth_bootstrap_complete: false }
    }

    pub fn is_authenticated(&self) -> bool {
        !self.user.discord_id.is_empty()
    }

    /// Load the stored session, fetch `/me`, mark bootstrap complete.
    pub fn bootstrap_auth(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let token = auth::get_token();
            if !token.is_empty() {
                let _ = this.update(cx, |s, _| s.token = token);
                let me = run_blocking(cx, api::me).await;
                let _ = this.update(cx, |s, cx| s.apply_me(me, cx));
            }
            let _ = this.update(cx, |s, cx| {
                s.auth_bootstrap_complete = true;
                cx.notify();
            });
        })
        .detach();
    }

    /// Persists the token received through the `classicaddonmanager://auth` deeplink, then fetches `/me`. Resolves
    /// once the token is stored, the `/me` fetch goes on in the background. A failure is returned unlogged.
    pub fn save_token(&mut self, token: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        if token.is_empty() {
            return Task::ready(Err(anyhow!("the sign-in link carried no token")));
        }
        let backend = backend(cx);
        cx.spawn(async move |this, cx| {
            let t = token.clone();
            run_blocking(cx, move || auth::save_to_disk(&backend.paths, &t)).await?;
            this.update(cx, |this, cx| {
                this.token = token;
                this.fetch_current_user(cx);
            })
        })
    }

    pub fn fetch_current_user(&mut self, cx: &mut Context<Self>) {
        if self.token.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let me = run_blocking(cx, api::me).await;
            this.update(cx, |this, cx| this.apply_me(me, cx))
        })
        .detach();
    }

    /// A rejected token signs out locally, other failures keep the session for the next attempt.
    fn apply_me(&mut self, me: Result<User, ApiError>, cx: &mut Context<Self>) {
        match me {
            Ok(user) => {
                let changed = user.discord_id != self.user.discord_id;
                self.user = user;
                if changed {
                    queries::invalidate_all(cx);
                }
                cx.notify();
                restore_subscribed_addons(cx);
            }
            Err(ApiError::Unauthorized) => {
                let backend = backend(cx);
                cx.spawn(async move |this, cx| {
                    if let Err(e) = run_blocking(cx, move || auth::delete_from_disk(&backend.paths)).await {
                        log::error!("Clearing the rejected session token failed: {e:#}");
                    }
                    this.update(cx, |this, cx| this.clear_local_auth_state(cx))
                })
                .detach();
            }
            Err(e) => log::error!("Fetching the signed-in user failed: {e:#}"),
        }
    }

    /// Clears the stored token, then the session in memory. Resolves to an error when the token could not be cleared,
    /// the session then persists. A failure is returned unlogged.
    pub fn sign_out(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let backend = backend(cx);
        cx.spawn(async move |this, cx| {
            run_blocking(cx, move || auth::delete_from_disk(&backend.paths)).await?;
            this.update(cx, |this, cx| this.clear_local_auth_state(cx))
        })
    }

    pub fn clear_local_auth_state(&mut self, cx: &mut Context<Self>) {
        let was_authed = self.is_authenticated();
        self.token.clear();
        self.user = User::default();
        if was_authed {
            queries::invalidate_all(cx);
        }
        cx.notify();
    }
}

/// After login: installs every subscribed addon that is missing locally and toasts the outcome.
fn restore_subscribed_addons(cx: &mut App) {
    let restore = super::addons(cx).update(cx, |s, cx| s.restore_subscribed(cx));
    cx.spawn(async move |cx| {
        let result = restore.await;
        cx.update(|cx| match result {
            Err(e) => {
                log::error!("Restoring subscribed addons failed: {e:#}");
                toast::error(cx, "Failed to fetch subscribed addons.");
            }
            Ok(Restored { installed, failed }) => {
                if installed > 0 {
                    toast::success_titled(
                        cx,
                        "Addons restored",
                        format!("{installed} addon(s) were restored from the server."),
                    );
                }
                if failed > 0 {
                    toast::error(cx, format!("{failed} addon(s) failed to restore. Check logfile for details."));
                }
            }
        });
    })
    .detach();
}
