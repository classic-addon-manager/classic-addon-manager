//! Wire types of the `/dev/addon/*` API, decoded with serde. The field rules that a plain derive doesn't express (whole
//! numbers, keys that must be present but may be `null`, non-empty strings, rows that are dropped instead of failing the
//! reply) are the `deserialize_with` functions in [`super::parse`].
use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::parse;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Widget {
    Text,
    Textarea,
    EnumMulti,
    StringList,
    Checkbox,
}

/// One declaration value. It is sent and read as the bare JSON string, string list or boolean.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WireValue {
    Text(String),
    List(Vec<String>),
    Bool(bool),
}

impl WireValue {
    /// The empty value of a widget: unchecked, empty text or an empty list.
    pub fn empty(widget: Widget) -> Self {
        match widget {
            Widget::Checkbox => WireValue::Bool(false),
            Widget::Text | Widget::Textarea => WireValue::Text(String::new()),
            Widget::EnumMulti | Widget::StringList => WireValue::List(Vec::new()),
        }
    }

    /// Whether this is the type of value `widget` edits.
    pub fn fits(&self, widget: Widget) -> bool {
        matches!(
            (self, widget),
            (WireValue::Bool(_), Widget::Checkbox)
                | (WireValue::Text(_), Widget::Text | Widget::Textarea)
                | (WireValue::List(_), Widget::EnumMulti | Widget::StringList)
        )
    }
}

/// A declaration's values by schema field key, sorted so equality and iteration are deterministic.
pub type DeclarationValues = BTreeMap<String, WireValue>;
/// Validation messages by field key.
pub type FieldErrors = HashMap<String, Vec<String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeclarationKind {
    New,
    Update,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LengthUnit {
    Runes,
    Bytes,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaField {
    #[serde(deserialize_with = "parse::non_empty")]
    pub key: String,
    pub label: String,
    pub section: String,
    pub widget: Widget,
    pub required: bool,
    pub hint: Option<String>,
    #[serde(default)]
    pub immutable: bool,
    #[serde(default)]
    pub half: bool,
    pub pattern: Option<String>,
    pub max_length: Option<usize>,
    pub length_unit: Option<LengthUnit>,
    #[serde(default, rename = "enum")]
    pub enum_values: Vec<String>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub item_pattern: Option<String>,
    pub joined_separator: Option<String>,
    pub joined_max_length: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddonSchema {
    pub fields: Vec<SchemaField>,
}

/// The schema document as the server serves it: one this app can edit with, or one whose version or widgets it
/// doesn't know. Decoded by [`parse`].
#[derive(Clone, Debug, PartialEq)]
pub enum SchemaDoc {
    Supported(AddonSchema),
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SourceAddon {
    #[serde(deserialize_with = "parse::non_empty")]
    pub uuid: String,
    #[serde(deserialize_with = "parse::non_empty")]
    pub name: String,
    /// Empty when the addon has no alias of its own.
    pub alias: String,
    /// Newer than the rest of the payload: missing or invalid reads as 0.
    #[serde(default, deserialize_with = "parse::whole_or_zero")]
    pub downloads: i64,
    /// Newer as well: missing or invalid reads as not enabled.
    #[serde(default, deserialize_with = "parse::or_default")]
    pub webhook_enabled: bool,
    /// Time of the last release webhook event for the addon's repo, `None` when none yet.
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub webhook_last_event_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SourceSubmission {
    #[serde(deserialize_with = "parse::whole")]
    pub id: i64,
    pub kind: DeclarationKind,
    #[serde(deserialize_with = "parse::non_empty")]
    pub name: String,
    #[serde(default, deserialize_with = "parse::non_empty_or_none")]
    pub status: Option<String>,
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
pub struct AddonSources {
    #[serde(deserialize_with = "parse::rows")]
    pub addons: Vec<SourceAddon>,
    #[serde(deserialize_with = "parse::rows")]
    pub submissions: Vec<SourceSubmission>,
    /// GitHub App install page for instant updates, empty when the API does not offer one over `https`.
    #[serde(default, deserialize_with = "parse::https_or_empty")]
    pub webhook_install_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubmissionStatus {
    Open,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SubmissionMessage {
    #[serde(deserialize_with = "parse::whole")]
    pub id: i64,
    pub body: String,
    #[serde(default, deserialize_with = "parse::whole_or_zero")]
    pub revision: i64,
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SubmissionDetail {
    #[serde(deserialize_with = "parse::whole")]
    pub id: i64,
    pub kind: DeclarationKind,
    pub status: SubmissionStatus,
    #[serde(default, deserialize_with = "parse::non_empty_or_none")]
    pub target_addon_uuid: Option<String>,
    #[serde(default, deserialize_with = "parse::whole_or_zero")]
    pub revision: i64,
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "parse::date_or_none")]
    pub rejected_at: Option<DateTime<Utc>>,
    /// Messages that don't parse are dropped, a missing list reads as none.
    #[serde(default, deserialize_with = "parse::rows_or_empty")]
    pub messages: Vec<SubmissionMessage>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EditorSource {
    New,
    Addon { uuid: String },
    Submission { id: i64 },
}

/// The stored declaration an [`EditorSource`] resolves to.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceValues {
    pub kind: DeclarationKind,
    pub values: DeclarationValues,
}

/// An icon the server staged: the id to submit it by and the URL to preview it at.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct UploadedIcon {
    #[serde(deserialize_with = "parse::non_empty")]
    pub asset_id: String,
    pub url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionLagBucket {
    Latest,
    OneBehind,
    TwoBehind,
    Unavailable,
}

impl VersionLagBucket {
    pub const ALL: [VersionLagBucket; 4] = [
        VersionLagBucket::Latest,
        VersionLagBucket::OneBehind,
        VersionLagBucket::TwoBehind,
        VersionLagBucket::Unavailable,
    ];
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SnapshotPoint {
    #[serde(deserialize_with = "parse::day")]
    pub taken_on: NaiveDate,
    #[serde(deserialize_with = "parse::whole")]
    pub downloads: i64,
    #[serde(deserialize_with = "parse::whole")]
    pub subscribers: i64,
    /// `null` when the day before has no snapshot. The key itself is required.
    #[serde(deserialize_with = "parse::nullable_whole")]
    pub daily_downloads: Option<i64>,
    #[serde(deserialize_with = "parse::nullable_whole")]
    pub daily_subscriber_change: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonDownloadStats {
    #[serde(deserialize_with = "parse::whole")]
    pub total: i64,
    pub series: Vec<SnapshotPoint>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonRatingStats {
    #[serde(deserialize_with = "parse::whole")]
    pub likes: i64,
    #[serde(deserialize_with = "parse::whole")]
    pub dislikes: i64,
    #[serde(deserialize_with = "parse::whole")]
    pub total_votes: i64,
    /// `null` exactly when there are no votes ([`parse::ratings`]). The key itself is required.
    #[serde(deserialize_with = "parse::nullable")]
    pub like_percentage: Option<f64>,
    #[serde(deserialize_with = "parse::whole")]
    pub deleted_accounts: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonSubscriberStats {
    #[serde(deserialize_with = "parse::whole")]
    pub current: i64,
    pub series: Vec<SnapshotPoint>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonVersionUsage {
    #[serde(deserialize_with = "parse::whole")]
    pub release_id: i64,
    pub tag_name: String,
    #[serde(deserialize_with = "parse::whole")]
    pub subscribers: i64,
    pub share: f64,
    pub lag_bucket: VersionLagBucket,
}

/// Subscribers per lag bucket. Every bucket is required.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct VersionBuckets {
    #[serde(deserialize_with = "parse::whole")]
    latest: i64,
    #[serde(deserialize_with = "parse::whole")]
    one_behind: i64,
    #[serde(deserialize_with = "parse::whole")]
    two_behind: i64,
    #[serde(deserialize_with = "parse::whole")]
    unavailable: i64,
}

impl VersionBuckets {
    pub fn count(&self, bucket: VersionLagBucket) -> i64 {
        match bucket {
            VersionLagBucket::Latest => self.latest,
            VersionLagBucket::OneBehind => self.one_behind,
            VersionLagBucket::TwoBehind => self.two_behind,
            VersionLagBucket::Unavailable => self.unavailable,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonVersionStats {
    pub buckets: VersionBuckets,
    pub share: Vec<AddonVersionUsage>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AddonDeveloperStats {
    pub downloads: AddonDownloadStats,
    #[serde(deserialize_with = "parse::ratings")]
    pub ratings: AddonRatingStats,
    pub subscribers: AddonSubscriberStats,
    pub versions: AddonVersionStats,
}

pub const SCHEMA_FIELD_KEYS: [&str; 11] =
    ["name", "alias", "description", "author", "repo", "branch", "tags", "keywords", "dependencies", "library", "kofi"];
