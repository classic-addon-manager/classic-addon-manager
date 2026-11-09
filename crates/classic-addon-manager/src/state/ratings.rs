//! Per-account addon rating: loading the current vote and saving a new one.
//!
//! ```ignore
//! // field: rating: Entity<RatingModel>
//! self.rating = cx.new(|cx| RatingModel::new(addon.name.clone(), addon.alias.clone(), addon.is_managed, cx));
//! cx.observe(&self.rating, |_, _, cx| cx.notify()).detach();
//! // render: let m = self.rating.read(cx); RatingButtons::new("r").rating(m.rating(cx).into()).disabled(m.busy(cx))
//! //     .on_rate({ let m = self.rating.clone(); move |v, _, cx| m.update(cx, |m, cx| m.rate(v, cx)) })
//! ```
//! Votes are one app-wide [`Query`] keyed by account and addon, reused for 5 minutes, so they survive page switches.
use std::time::Duration;

use cam_core::api;
use cam_ui::icons;
use cam_ui::toast;
use gpui_kit::{App, Context, Entity, Subscription, Task};

use super::{query::Query, tasks::run_blocking};

const STALE: Duration = Duration::from_secs(5 * 60);

/// The signed-in account's vote on an addon (1 liked, -1 disliked, 0 none), by `(discord_id, addon name)`.
pub type RatingQuery = Query<(String, String), i8>;

pub(super) fn query() -> RatingQuery {
    RatingQuery::blocking(|(_, name)| format!("your rating of {name}"), STALE, |(_, name)| Ok(api::my_rating(name)?))
}

pub struct RatingModel {
    name: String,
    alias: String,
    enabled: bool,
    discord_id: String,
    votes: Entity<RatingQuery>,
    /// True while a new vote is being saved.
    is_saving: bool,
    _subs: [Subscription; 2],
}

impl RatingModel {
    pub fn new(name: String, alias: String, enabled: bool, cx: &mut Context<Self>) -> Self {
        let user = super::user(cx);
        let votes = super::queries::rating(cx);
        let discord_id = user.read(cx).user.discord_id.clone();
        let subs = [
            cx.observe(&user, |this, user, cx| {
                let id = user.read(cx).user.discord_id.clone();
                if id != this.discord_id {
                    this.discord_id = id;
                    this.is_saving = false;
                    this.load(cx);
                    cx.notify();
                }
            }),
            cx.observe(&votes, |_, _, cx| cx.notify()),
        ];
        let this = Self { name, alias, enabled, discord_id, votes, is_saving: false, _subs: subs };
        this.load(cx);
        this
    }

    fn active(&self) -> bool {
        self.enabled && !self.discord_id.is_empty()
    }

    fn key(&self) -> (String, String) {
        (self.discord_id.clone(), self.name.clone())
    }

    fn load(&self, cx: &mut Context<Self>) {
        if self.active() {
            self.votes.update(cx, |q, cx| q.ensure(self.key(), cx));
        }
    }

    /// Current vote: 1 liked, -1 disliked, 0 none (or not loaded yet).
    pub fn rating(&self, cx: &App) -> i8 {
        if !self.active() {
            return 0;
        }
        self.votes.read(cx).value(&self.key()).copied().unwrap_or(0)
    }

    /// True while the vote is being fetched or saved.
    pub fn busy(&self, cx: &App) -> bool {
        self.is_saving || (self.active() && self.votes.read(cx).get(&self.key()).is_pending())
    }

    /// Saves a vote of 1 (like) or -1 (dislike) and toasts the outcome. Resolves to true when the vote was saved for
    /// the account that is still signed in, which is the only case where the emoji burst plays.
    pub fn rate(&mut self, new_rating: i8, cx: &mut Context<Self>) -> Task<bool> {
        if !self.active() || !matches!(new_rating, -1 | 1) || self.rating(cx) == new_rating || self.is_saving {
            return Task::ready(false);
        }
        self.is_saving = true;
        cx.notify();
        let key = self.key();
        let name = self.name.clone();
        let (verb, icon) = if new_rating == 1 { ("liked", icons::THUMBS_UP) } else { ("disliked", icons::THUMBS_DOWN) };
        cx.spawn(async move |this, cx| {
            let res = run_blocking(cx, move || api::rate(&name, new_rating == 1)).await;
            this.update(cx, |this, cx| {
                this.is_saving = false;
                cx.notify();
                let same_account = this.discord_id == key.0;
                match res {
                    Ok(()) if same_account => {
                        // Show the vote at once, then read the authoritative one back from the server.
                        this.votes.update(cx, |q, cx| {
                            q.set(key.clone(), new_rating, cx);
                            q.refetch(key, cx).detach();
                        });
                        let rated = toast::Toast::new("Addon rated", format!("You {verb} {}", this.alias)).icon(icon);
                        toast::show(cx, rated);
                        true
                    }
                    Ok(()) => false,
                    Err(e) => {
                        log::error!("Rating {} failed: {e:#}", this.name);
                        if same_account {
                            toast::error_titled(cx, "Error", "Failed to rate addon, try again later");
                        }
                        false
                    }
                }
            })
            .unwrap_or(false)
        })
    }
}
