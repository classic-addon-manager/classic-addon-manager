//! The owner's list model: addons and submissions as the Developer page lists them, plus the review status of each.
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::types::*;
use crate::backend::shared::AddonManifest;

/// Visual family of a submission history status: drives the chip color and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewHistoryTone {
    Review,
    Approved,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewHistoryEntry {
    pub submission_id: i64,
    pub status: String,
    pub tone: ReviewHistoryTone,
    pub date: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedAddon {
    pub uuid: String,
    pub name: String,
    pub alias: String,
    pub repo: String,
    pub branch: Option<String>,
    pub icon: Option<String>,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub downloads: i64,
    pub like_percentage: Option<f64>,
    pub warning: Option<String>,
    pub library: bool,
    pub added_at: Option<DateTime<Utc>>,
    pub review_history: Vec<ReviewHistoryEntry>,
    pub webhook_enabled: bool,
    pub webhook_last_event_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedSubmissionStatus {
    Open,
    Rejected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedSubmission {
    pub id: i64,
    pub kind: DeclarationKind,
    pub status: OwnedSubmissionStatus,
    pub created_at: Option<DateTime<Utc>>,
    /// Name of the addon the submission is for, also the submission's title in the list.
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct OwnedAddonsData {
    pub addons: Vec<OwnedAddon>,
    pub submissions: Vec<OwnedSubmission>,
    pub webhook_install_url: String,
}

/// The owner's addons and submissions: the sources the server lists, with the details of the catalog for every addon
/// it has an entry for.
pub fn sources_to_owned(sources: &AddonSources, manifests: &[AddonManifest]) -> OwnedAddonsData {
    let manifest_by_name: HashMap<&str, &AddonManifest> = manifests.iter().map(|m| (m.name.as_str(), m)).collect();
    let published: HashSet<&str> = sources.addons.iter().map(|a| a.name.as_str()).collect();
    // Submissions per addon name, with the names kept in the order they first appear.
    let mut history_order: Vec<&str> = Vec::new();
    let mut history_by_name: HashMap<&str, Vec<&SourceSubmission>> = HashMap::new();
    let mut standalone: Vec<OwnedSubmission> = Vec::new();

    for submission in &sources.submissions {
        if is_standalone_submission(submission.kind, &submission.name, &published) {
            standalone.push(to_owned_submission(submission));
        } else {
            let list = history_by_name.entry(submission.name.as_str()).or_insert_with(|| {
                history_order.push(submission.name.as_str());
                Vec::new()
            });
            list.push(submission);
        }
    }

    let addons = sources
        .addons
        .iter()
        .map(|addon| {
            to_owned_addon(
                addon,
                manifest_by_name.get(addon.name.as_str()).copied(),
                history_by_name.get(addon.name.as_str()).map(Vec::as_slice).unwrap_or(&[]),
            )
        })
        .collect();
    let mut submissions = standalone;
    for name in history_order {
        submissions.extend(history_by_name[name].iter().map(|s| to_owned_submission(s)));
    }
    OwnedAddonsData { addons, submissions, webhook_install_url: sources.webhook_install_url.clone() }
}

/// Update submissions for an already published addon belong to that addon's review history, so they are not
/// listed or counted on their own.
pub fn is_standalone_submission(kind: DeclarationKind, name: &str, published_names: &HashSet<&str>) -> bool {
    !(kind == DeclarationKind::Update && published_names.contains(name))
}

impl OwnedSubmission {
    pub fn is_standalone(&self, published_names: &HashSet<&str>) -> bool {
        is_standalone_submission(self.kind, &self.name, published_names)
    }
}

fn to_owned_addon(
    addon: &SourceAddon,
    manifest: Option<&AddonManifest>,
    submissions: &[&SourceSubmission],
) -> OwnedAddon {
    OwnedAddon {
        uuid: addon.uuid.clone(),
        name: addon.name.clone(),
        alias: if addon.alias.is_empty() { addon.name.clone() } else { addon.alias.clone() },
        repo: manifest.map(|m| m.repo.clone()).unwrap_or_default(),
        branch: manifest.map(|m| m.branch.clone()).filter(|b| !b.is_empty()),
        icon: manifest.and_then(|m| m.icon.clone()),
        author: String::new(),
        description: String::new(),
        tags: Vec::new(),
        downloads: addon.downloads,
        like_percentage: None,
        warning: None,
        library: manifest.map(|m| m.library).unwrap_or(false),
        added_at: None,
        review_history: to_review_history(submissions),
        webhook_enabled: addon.webhook_enabled,
        webhook_last_event_at: addon.webhook_last_event_at,
    }
}

fn to_review_history(submissions: &[&SourceSubmission]) -> Vec<ReviewHistoryEntry> {
    let mut sorted: Vec<&&SourceSubmission> = submissions.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.id));
    sorted
        .into_iter()
        .map(|s| ReviewHistoryEntry {
            submission_id: s.id,
            status: review_status(s.status.as_deref()),
            tone: status_tone(s.status.as_deref()),
            date: s.created_at,
        })
        .collect()
}

/// Tone for a submission history row. A status the backend sends that we do not map stays visibly distinct.
pub fn status_tone(status: Option<&str>) -> ReviewHistoryTone {
    match status {
        // A missing status cannot be asserted as a real outcome, so it stays unknown.
        None | Some("") => ReviewHistoryTone::Unknown,
        Some("open") | Some("in_review") => ReviewHistoryTone::Review,
        Some("rejected") => ReviewHistoryTone::Rejected,
        Some("approved") => ReviewHistoryTone::Approved,
        Some(_) => ReviewHistoryTone::Unknown,
    }
}

fn review_status(status: Option<&str>) -> String {
    match status {
        None | Some("") => "Unknown".into(),
        Some("open") | Some("in_review") => "In review".into(),
        Some("rejected") => "Rejected".into(),
        Some("approved") => "Approved".into(),
        Some(other) => other.into(),
    }
}

fn to_owned_submission(submission: &SourceSubmission) -> OwnedSubmission {
    OwnedSubmission {
        id: submission.id,
        kind: submission.kind,
        status: if submission.status.as_deref() == Some("rejected") {
            OwnedSubmissionStatus::Rejected
        } else {
            OwnedSubmissionStatus::Open
        },
        created_at: submission.created_at,
        name: submission.name.clone(),
    }
}

// ---- review status ----------------------------------------------------------------------------------------------

/// Catalog-review outcome the addon's latest submission can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewOutcome {
    InReview,
    Approved,
    Rejected,
}

/// The addon's current catalog-review status, derived from its newest submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddonReview {
    /// Submission whose detail shows the real proposed declaration and feedback.
    pub submission_id: i64,
    pub status: ReviewOutcome,
}

/// Current review status from the addon's latest submission. History is newest-first; an empty history or an
/// unmapped status yields no review.
pub fn latest_review(history: &[ReviewHistoryEntry]) -> Option<AddonReview> {
    let latest = history.first()?;
    let status = match latest.tone {
        ReviewHistoryTone::Review => ReviewOutcome::InReview,
        ReviewHistoryTone::Approved => ReviewOutcome::Approved,
        ReviewHistoryTone::Rejected => ReviewOutcome::Rejected,
        // An unmapped backend status must never be presented as a known outcome.
        ReviewHistoryTone::Unknown => return None,
    };
    Some(AddonReview { submission_id: latest.submission_id, status })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(
        id: i64,
        kind: DeclarationKind,
        name: &str,
        status: Option<&str>,
        created: Option<&str>,
    ) -> SourceSubmission {
        let created_at = created.map(|d| DateTime::parse_from_rfc3339(d).unwrap().with_timezone(&Utc));
        SourceSubmission { id, kind, name: name.into(), status: status.map(Into::into), created_at }
    }

    fn addon(name: &str) -> SourceAddon {
        SourceAddon {
            uuid: format!("uuid-{name}"),
            name: name.into(),
            alias: name.into(),
            downloads: 5,
            webhook_enabled: false,
            webhook_last_event_at: None,
        }
    }

    fn manifest(name: &str, repo: &str, branch: &str, icon: Option<&str>, library: bool) -> AddonManifest {
        serde_json::from_value(serde_json::json!({
            "name": name, "repo": repo, "branch": branch, "icon": icon, "library": library
        }))
        .unwrap()
    }

    #[test]
    fn tones_and_labels() {
        assert_eq!(status_tone(None), ReviewHistoryTone::Unknown);
        assert_eq!(status_tone(Some("")), ReviewHistoryTone::Unknown);
        assert_eq!(status_tone(Some("open")), ReviewHistoryTone::Review);
        assert_eq!(status_tone(Some("in_review")), ReviewHistoryTone::Review);
        assert_eq!(status_tone(Some("approved")), ReviewHistoryTone::Approved);
        assert_eq!(status_tone(Some("rejected")), ReviewHistoryTone::Rejected);
        assert_eq!(status_tone(Some("weird")), ReviewHistoryTone::Unknown);
        assert_eq!(review_status(None), "Unknown");
        assert_eq!(review_status(Some("open")), "In review");
        assert_eq!(review_status(Some("weird")), "weird");
    }

    #[test]
    fn groups_history_and_standalone() {
        let sources = AddonSources {
            addons: vec![addon("A")],
            submissions: vec![
                sub(1, DeclarationKind::New, "A", Some("approved"), Some("2026-01-01T00:00:00Z")),
                sub(4, DeclarationKind::Update, "A", Some("open"), Some("2026-03-01T00:00:00Z")),
                sub(2, DeclarationKind::Update, "A", Some("rejected"), None),
                sub(3, DeclarationKind::New, "B", Some("open"), None),
                sub(5, DeclarationKind::Update, "C", None, None),
            ],
            webhook_install_url: "https://x".into(),
        };
        let manifests = [manifest("A", "me/a", "main", Some("https://cdn/a.png"), true)];
        let owned = sources_to_owned(&sources, &manifests);
        assert_eq!(owned.webhook_install_url, "https://x");
        let a = &owned.addons[0];
        assert_eq!(a.repo, "me/a");
        assert_eq!(a.branch.as_deref(), Some("main"));
        assert_eq!(a.icon.as_deref(), Some("https://cdn/a.png"));
        assert!(a.library);
        // only update submissions for published names are history, newest first
        assert_eq!(a.review_history.iter().map(|h| h.submission_id).collect::<Vec<_>>(), vec![4, 2]);
        assert_eq!(a.review_history[0].status, "In review");
        assert_eq!(a.review_history[1].status, "Rejected");
        assert_eq!(a.review_history[1].date, None);
        assert_eq!(a.review_history[0].date, owned.submissions[3].created_at);
        assert!(a.review_history[0].date.is_some());
        // standalone first (in order), then the history submissions
        assert_eq!(owned.submissions.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 3, 5, 4, 2]);
        let published: HashSet<&str> = ["A"].into_iter().collect();
        let visible: Vec<i64> =
            owned.submissions.iter().filter(|s| s.is_standalone(&published)).map(|s| s.id).collect();
        assert_eq!(visible, vec![1, 3, 5]);
        assert_eq!(owned.submissions[2].status, OwnedSubmissionStatus::Open);
        assert_eq!(owned.submissions[4].status, OwnedSubmissionStatus::Rejected);
    }

    #[test]
    fn addon_without_manifest() {
        let sources =
            AddonSources { addons: vec![SourceAddon { alias: String::new(), ..addon("A") }], ..Default::default() };
        let owned = sources_to_owned(&sources, &[]);
        let a = &owned.addons[0];
        // an addon without an alias shows its name
        assert_eq!(a.alias, "A");
        assert_eq!(a.repo, "");
        assert_eq!(a.branch, None);
        assert_eq!(a.icon, None);
        assert!(!a.library);
        // empty branch in the manifest is treated as missing
        let owned = sources_to_owned(&sources, &[manifest("A", "r", "", None, false)]);
        assert_eq!(owned.addons[0].branch, None);
    }

    #[test]
    fn latest_review_mapping() {
        let entry = |id, tone| ReviewHistoryEntry { submission_id: id, status: String::new(), tone, date: None };
        assert_eq!(latest_review(&[]), None);
        assert_eq!(latest_review(&[entry(9, ReviewHistoryTone::Unknown), entry(8, ReviewHistoryTone::Review)]), None);
        assert_eq!(
            latest_review(&[entry(9, ReviewHistoryTone::Review)]),
            Some(AddonReview { submission_id: 9, status: ReviewOutcome::InReview })
        );
        assert_eq!(latest_review(&[entry(9, ReviewHistoryTone::Approved)]).unwrap().status, ReviewOutcome::Approved);
        assert_eq!(latest_review(&[entry(9, ReviewHistoryTone::Rejected)]).unwrap().status, ReviewOutcome::Rejected);
    }
}
