//! Decoding rules for the `/dev/addon/*` replies that serde's derive doesn't express. The `deserialize_with` functions
//! hold the field rules of [`super::types`]. The rest checks a reply as a whole: the schema's version and widgets,
//! stored values read through the schema, the validation verdict and the acknowledgement of a write. Decoding is strict
//! except where the API is lenient: list rows that don't parse are dropped, and fields added to the API later fall back
//! to a default.
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, Error},
};
use serde_json::{Map, Number, Value};

use super::types::*;
use crate::api::FieldError;

// ---- field rules ----------------------------------------------------------------------------------------------------

/// The integer a JSON number holds: `3` or `3.0`, not `1.5`.
fn whole_of(n: &Number) -> Option<i64> {
    n.as_i64().or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64))
}

fn to_whole<E: Error>(n: Number) -> Result<i64, E> {
    whole_of(&n).ok_or_else(|| E::custom(format!("{n} is not a whole number")))
}

/// A whole number, written as an integer or as a float without a fraction.
pub fn whole<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    to_whole(Number::deserialize(d)?)
}

/// A whole number or `null`. Fields using it have no `default`, so the key itself is required.
pub fn nullable_whole<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    Option::<Number>::deserialize(d)?.map(to_whole).transpose()
}

/// A value or `null`, for keys that must be present. A plain `Option` field would also accept a missing key.
pub fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

/// A whole number, 0 when it is anything else.
pub fn whole_or_zero<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Number(n) => whole_of(&n).unwrap_or(0),
        _ => 0,
    })
}

/// The value, or its type's default when it has another shape.
pub fn or_default<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(d: D) -> Result<T, D::Error> {
    Ok(T::deserialize(Value::deserialize(d)?).unwrap_or_default())
}

pub fn non_empty<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if s.is_empty() { Err(D::Error::custom("empty string")) } else { Ok(s) }
}

/// A non-empty string, `None` for anything else.
pub fn non_empty_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(s) if !s.is_empty() => Some(s),
        _ => None,
    })
}

/// An `https://` URL, empty for anything else.
pub fn https_or_empty<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(s) if s.starts_with("https://") => s,
        _ => String::new(),
    })
}

/// A time in one of the shapes [`parse_date`] reads, `None` for anything else.
pub fn date_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(s) => parse_date(&s),
        _ => None,
    })
}

/// A day written exactly as `YYYY-MM-DD`.
pub fn day<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDate, D::Error> {
    let s = String::deserialize(d)?;
    let b = s.as_bytes();
    let shaped = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
    if !shaped {
        return Err(D::Error::custom(format!("{s:?} is not a YYYY-MM-DD day")));
    }
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(D::Error::custom)
}

/// A list whose rows are dropped when they don't parse. The list itself is required.
pub fn rows<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Vec::<Value>::deserialize(d)?.into_iter().filter_map(|row| T::deserialize(row).ok()).collect())
}

/// Like [`rows`], and anything that isn't a list reads as an empty one.
pub fn rows_or_empty<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Array(items) => items.into_iter().filter_map(|row| T::deserialize(row).ok()).collect(),
        _ => Vec::new(),
    })
}

/// Ratings whose like percentage is `null` exactly when there are no votes.
pub fn ratings<'de, D: Deserializer<'de>>(d: D) -> Result<AddonRatingStats, D::Error> {
    let ratings = AddonRatingStats::deserialize(d)?;
    if ratings.like_percentage.is_some() != (ratings.total_votes != 0) {
        return Err(D::Error::custom("like_percentage must be null exactly when there are no votes"));
    }
    Ok(ratings)
}

/// Parses the date shapes the API sends (RFC 3339, `YYYY-MM-DD[ T]HH:MM:SS`, `YYYY-MM-DD`). Text without an offset is
/// read as UTC, anything else yields `None`.
fn parse_date(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(value) {
        return Some(d.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(value, fmt) {
            return Some(d.and_utc());
        }
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0)).map(|d| d.and_utc())
}

// ---- whole replies --------------------------------------------------------------------------------------------------

/// Only version 2 is read: any other version is [`SchemaDoc::Unsupported`], whatever the rest holds. So is a widget
/// this app doesn't know, in the widget list or on a field. A field with an unknown widget isn't read further, since a
/// new widget may come with options of its own. Fields are read in order, so the first field the app can't use decides
/// between unsupported and malformed.
impl<'de> Deserialize<'de> for SchemaDoc {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Document {
            version: f64,
            widgets: Option<Value>,
            fields: Option<Value>,
        }
        let doc = Document::deserialize(d)?;
        if doc.version != 2.0 {
            return Ok(SchemaDoc::Unsupported);
        }
        let (Some(Value::Array(widgets)), Some(Value::Array(fields))) = (doc.widgets, doc.fields) else {
            return Err(D::Error::custom("schema without widget and field lists"));
        };
        if widgets.iter().any(|w| Widget::deserialize(w).is_err()) {
            return Ok(SchemaDoc::Unsupported);
        }
        let mut parsed = Vec::with_capacity(fields.len());
        for field in fields {
            if field.get("widget").is_some_and(|w| w.is_string() && Widget::deserialize(w).is_err()) {
                return Ok(SchemaDoc::Unsupported);
            }
            parsed.push(SchemaField::deserialize(field).map_err(D::Error::custom)?);
        }
        Ok(SchemaDoc::Supported(AddonSchema { fields: parsed }))
    }
}

/// The stored values of one editor source as the server sends them. Their types depend on the schema, so they are
/// read through it with [`StoredValues::read`].
#[derive(Deserialize)]
pub struct StoredValues {
    kind: DeclarationKind,
    values: Map<String, Value>,
    icon_url: Option<String>,
}

impl StoredValues {
    /// Every schema field gets a value of its widget's type (`null` and absent read as empty), keys outside the schema
    /// are dropped, and the icon URL is added as `icon_url`. Fails on a value of another type than its widget's.
    pub fn read(mut self, schema: &AddonSchema) -> Result<SourceValues, String> {
        let mut values = DeclarationValues::new();
        for field in &schema.fields {
            let value = match self.values.remove(&field.key) {
                None | Some(Value::Null) => WireValue::empty(field.widget),
                Some(raw) => WireValue::deserialize(raw)
                    .ok()
                    .filter(|v| v.fits(field.widget))
                    .ok_or_else(|| format!("the value of {} doesn't fit its widget", field.key))?,
            };
            values.insert(field.key.clone(), value);
        }
        values.insert("icon_url".into(), WireValue::Text(self.icon_url.unwrap_or_default()));
        Ok(SourceValues { kind: self.kind, values })
    }
}

/// The verdict of a validation. A failed one lists the problems the server could attribute.
#[derive(Deserialize)]
pub struct Verdict {
    pub valid: bool,
    /// `{field, message}` entries. Entries of any other shape are dropped.
    #[serde(default, deserialize_with = "rows_or_empty")]
    pub errors: Vec<FieldError>,
}

/// The reply to a stored or withdrawn submission. It must carry the revision as well.
#[derive(Deserialize)]
pub struct Acknowledgement {
    #[serde(deserialize_with = "whole")]
    pub id: i64,
    #[serde(rename = "revision", deserialize_with = "whole")]
    _revision: i64,
    #[serde(default, deserialize_with = "non_empty_or_none")]
    pub status: Option<String>,
}

/// Splits field errors into those the form shows on a field (keys in `schema_keys`) and the rest, which have no
/// field to sit on.
pub fn group_validation_errors(errors: &[FieldError], schema_keys: &[&str]) -> (FieldErrors, Vec<String>) {
    let mut fields = FieldErrors::new();
    let mut other = Vec::new();
    for FieldError { field, message } in errors {
        if schema_keys.contains(&field.as_str()) {
            fields.entry(field.clone()).or_default().push(message.clone());
            continue;
        }
        other.push(message.clone());
    }
    (fields, other)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn decode<T: DeserializeOwned>(data: Value) -> Option<T> {
        serde_json::from_value(data).ok()
    }

    fn schema() -> AddonSchema {
        let data = json!({
            "version": 2,
            "widgets": ["text", "textarea", "enum-multi", "string-list", "checkbox"],
            "fields": [
                {"key": "name", "label": "Name", "section": "Basics", "widget": "text", "required": true, "immutable": true, "maxLength": 40, "lengthUnit": "runes"},
                {"key": "tags", "label": "Tags", "section": "Basics", "widget": "enum-multi", "required": false, "enum": ["ui", "bags"], "maxItems": 3},
                {"key": "keywords", "label": "Keywords", "section": "Basics", "widget": "string-list", "required": false},
                {"key": "library", "label": "Library", "section": "Basics", "widget": "checkbox", "required": false}
            ]
        });
        match decode(data) {
            Some(SchemaDoc::Supported(s)) => s,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn schema_ok_and_options() {
        let s = schema();
        assert_eq!(s.fields.len(), 4);
        assert!(s.fields[0].immutable);
        assert!(!s.fields[1].immutable);
        assert_eq!(s.fields[0].max_length, Some(40));
        assert_eq!(s.fields[0].length_unit, Some(LengthUnit::Runes));
        assert_eq!(s.fields[1].enum_values, ["ui", "bags"]);
        assert_eq!(s.fields[1].max_items, Some(3));
        assert!(s.fields[2].enum_values.is_empty());
    }

    #[test]
    fn schema_unsupported_is_not_malformed() {
        let field = |key: &str, widget: &str| json!({"key": key, "label": "A", "section": "S", "widget": widget, "required": false});
        let doc = |widgets: Value, fields: Value| json!({"version": 2, "widgets": widgets, "fields": fields});
        // another version is unsupported whatever else the document holds
        assert_eq!(decode(json!({"version": 3, "widgets": [], "fields": []})), Some(SchemaDoc::Unsupported));
        assert_eq!(decode(json!({"version": 3})), Some(SchemaDoc::Unsupported));
        assert_eq!(decode(doc(json!(["slider"]), json!([]))), Some(SchemaDoc::Unsupported));
        assert_eq!(decode(doc(json!([3]), json!([]))), Some(SchemaDoc::Unsupported));
        assert_eq!(decode(doc(json!(["text"]), json!([field("a", "slider")]))), Some(SchemaDoc::Unsupported));
        // a field with an unknown widget isn't read further: its options may be new
        let new_options = json!([{"key": "a", "label": "A", "section": "S", "widget": "slider", "required": false, "maxLength": "x"}]);
        assert_eq!(decode(doc(json!(["text"]), new_options)), Some(SchemaDoc::Unsupported));
        assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), json!([field("", "text")]))), None);
        let bad_option =
            json!([{"key": "a", "label": "A", "section": "S", "widget": "text", "required": false, "hint": 3}]);
        assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), bad_option)), None);
        let fractional =
            json!([{"key": "a", "label": "A", "section": "S", "widget": "text", "required": false, "maxLength": 1.5}]);
        assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), fractional)), None);
        // a `null` option reads as absent, but `immutable`, `half` and `enum` have defaults only when left out
        let null_options = json!([{"key": "a", "label": "A", "section": "S", "widget": "text", "required": false, "hint": null, "maxLength": null, "lengthUnit": null}]);
        let Some(SchemaDoc::Supported(s)) = decode(doc(json!(["text"]), null_options)) else { panic!() };
        assert_eq!((s.fields[0].hint.as_deref(), s.fields[0].max_length, s.fields[0].length_unit), (None, None, None));
        for key in ["immutable", "half", "enum"] {
            let mut entry = field("a", "text");
            entry[key] = Value::Null;
            assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), json!([entry]))), None, "{key}: null");
        }
        assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), json!([{"key": "a", "widget": 3}]))), None);
        // fields are read in order: the first one the app can't use decides
        let unsupported_first = json!([field("a", "slider"), field("", "text")]);
        assert_eq!(decode(doc(json!(["text"]), unsupported_first)), Some(SchemaDoc::Unsupported));
        let malformed_first = json!([field("", "text"), field("a", "slider")]);
        assert_eq!(decode::<SchemaDoc>(doc(json!(["text"]), malformed_first)), None);
        assert_eq!(decode::<SchemaDoc>(json!({"version": 2, "widgets": []})), None);
        assert_eq!(decode::<SchemaDoc>(json!({"version": "2", "widgets": [], "fields": []})), None);
        assert_eq!(decode::<SchemaDoc>(json!("nope")), None);
    }

    #[test]
    fn sources_filter_invalid_rows() {
        let data = json!({
            "addons": [
                {"uuid": "u1", "name": "A", "alias": "", "downloads": 3.0, "webhook_enabled": true, "webhook_last_event_at": "2026-01-02T03:04:05Z"},
                {"uuid": "", "name": "B", "alias": "b"},
                {"uuid": "u3", "name": "C", "alias": "c", "downloads": "x", "webhook_enabled": "yes", "webhook_last_event_at": "garbage"},
                {"uuid": "u4", "name": "D", "alias": null},
                "nope"
            ],
            "submissions": [
                {"id": 1, "kind": "new", "name": "A", "status": "open", "created_at": "2026-01-01T00:00:00Z"},
                {"id": 2, "kind": "weird", "name": "A"},
                {"id": 3, "kind": "update", "name": "", "status": "open"},
                {"id": 4, "kind": "update", "name": "C", "status": "", "created_at": "not a date"},
                {"id": 5.5, "kind": "new", "name": "E"}
            ],
            "webhook_install_url": "http://nope"
        });
        let s: AddonSources = decode(data).unwrap();
        assert_eq!(s.addons.iter().map(|a| a.uuid.as_str()).collect::<Vec<_>>(), ["u1", "u3"]);
        assert_eq!(s.addons[0].alias, "");
        assert_eq!(s.addons[0].downloads, 3);
        assert!(s.addons[0].webhook_enabled);
        assert_eq!(s.addons[0].webhook_last_event_at, parse_date("2026-01-02T03:04:05Z"));
        assert_eq!(s.addons[1].downloads, 0);
        assert_eq!(s.addons[1].webhook_last_event_at, None);
        assert!(!s.addons[1].webhook_enabled);
        assert_eq!(s.submissions.iter().map(|s| s.id).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(s.submissions[0].created_at, parse_date("2026-01-01T00:00:00Z"));
        assert_eq!(s.submissions[1].status, None);
        assert_eq!(s.submissions[1].created_at, None);
        assert_eq!(s.webhook_install_url, "");
        let ok_url = json!({"addons": [], "submissions": [], "webhook_install_url": "https://github.com/apps/x"});
        assert_eq!(decode::<AddonSources>(ok_url).unwrap().webhook_install_url, "https://github.com/apps/x");
        assert_eq!(decode::<AddonSources>(json!({"addons": []})), None);
        assert_eq!(decode::<AddonSources>(json!({"addons": null, "submissions": []})), None);
    }

    fn values(data: Value) -> Option<SourceValues> {
        decode::<StoredValues>(data)?.read(&schema()).ok()
    }

    #[test]
    fn values_map_by_schema() {
        let data = json!({
            "kind": "update",
            "values": {"name": "Foo", "tags": ["ui"], "library": null, "extra": 5},
            "icon_url": null
        });
        let SourceValues { kind, values: v } = values(data).unwrap();
        assert_eq!(kind, DeclarationKind::Update);
        assert_eq!(v["name"], WireValue::Text("Foo".into()));
        assert_eq!(v["tags"], WireValue::List(vec!["ui".into()]));
        assert_eq!(v["keywords"], WireValue::List(vec![]));
        assert_eq!(v["library"], WireValue::Bool(false));
        assert_eq!(v["icon_url"], WireValue::Text(String::new()));
        assert!(!v.contains_key("extra"));
        let with_icon = values(json!({"kind": "new", "values": {}, "icon_url": "https://x/i.png"})).unwrap();
        assert_eq!(with_icon.values["icon_url"], WireValue::Text("https://x/i.png".into()));

        assert_eq!(values(json!({"kind": "new", "values": {"name": 5}})), None);
        assert_eq!(values(json!({"kind": "new", "values": {"name": ["Foo"]}})), None);
        assert_eq!(values(json!({"kind": "new", "values": {"library": "yes"}})), None);
        assert_eq!(values(json!({"kind": "new", "values": {"tags": ["ui", 3]}})), None);
        assert_eq!(values(json!({"kind": "new", "values": {}, "icon_url": 4})), None);
        assert_eq!(values(json!({"kind": "x", "values": {}})), None);
        assert_eq!(values(json!({"kind": "new"})), None);
    }

    #[test]
    fn declaration_values_serialize_as_bare_json() {
        let mut v = DeclarationValues::new();
        v.insert("name".into(), WireValue::Text("Foo".into()));
        v.insert("tags".into(), WireValue::List(vec!["ui".into(), "bags".into()]));
        v.insert("library".into(), WireValue::Bool(true));
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"library":true,"name":"Foo","tags":["ui","bags"]}"#);
    }

    const KEYS: [&str; 3] = ["name", "tags", "icon"];

    #[test]
    fn validation_verdicts() {
        let data = json!({"valid": false, "errors": [
            {"field": "name", "message": "too short"},
            {"field": "name", "message": "bad chars"},
            {"field": "nonsense", "message": "other thing"},
            {"field": 3, "message": "skipped"}
        ]});
        let verdict: Verdict = decode(data).unwrap();
        assert!(!verdict.valid);
        assert_eq!(verdict.errors.len(), 3);
        let (fields, other) = group_validation_errors(&verdict.errors, &KEYS);
        assert_eq!(fields["name"], vec!["too short", "bad chars"]);
        assert_eq!(other, vec!["other thing"]);
        assert!(decode::<Verdict>(json!({"valid": true})).unwrap().valid);
        // invalid without a usable error list is still a failed verdict, never a pass
        let bare = decode::<Verdict>(json!({"valid": false, "errors": null})).unwrap();
        assert!(!bare.valid && bare.errors.is_empty());
        assert!(decode::<Verdict>(json!({"valid": "yes"})).is_none());
        assert!(decode::<Verdict>(json!({})).is_none());
    }

    #[test]
    fn saved_and_withdrawn_acknowledgements() {
        let saved: Acknowledgement = decode(json!({"id": 12, "revision": 2})).unwrap();
        assert_eq!((saved.id, saved.status), (12, None));
        assert!(decode::<Acknowledgement>(json!({"id": "x", "revision": 1})).is_none());
        assert!(decode::<Acknowledgement>(json!({"id": 12})).is_none());
        assert_eq!(decode::<Acknowledgement>(json!({"id": 12.0, "revision": 2.0})).map(|a| a.id), Some(12));
        assert!(decode::<Acknowledgement>(json!({"id": 12.5, "revision": 2})).is_none());
        assert!(decode::<Acknowledgement>(json!({"id": 12, "revision": 2.5})).is_none());
        let withdrawn: Acknowledgement = decode(json!({"id": 5, "revision": 3, "status": "withdrawn"})).unwrap();
        assert_eq!(withdrawn.status.as_deref(), Some("withdrawn"));
        assert!(decode::<Acknowledgement>(json!(null)).is_none());
    }

    #[test]
    fn submission_detail() {
        let data = json!({
            "id": 7, "kind": "update", "status": "rejected", "target_addon_uuid": "", "revision": 2,
            "created_at": "2026-01-01T00:00:00Z", "approved_at": null, "rejected_at": "2026-01-03T00:00:00Z",
            "messages": [
                {"id": 1, "body": "fix it", "revision": 1, "created_at": "2026-01-02T00:00:00Z"},
                {"id": "x", "body": "skipped"},
                {"id": 2, "body": "again"}
            ]
        });
        let d: SubmissionDetail = decode(data).unwrap();
        assert_eq!(d.id, 7);
        assert_eq!(d.kind, DeclarationKind::Update);
        assert_eq!(d.status, SubmissionStatus::Rejected);
        assert_eq!(d.target_addon_uuid, None);
        assert_eq!(d.revision, 2);
        assert_eq!(d.created_at, parse_date("2026-01-01T00:00:00Z"));
        assert_eq!(d.approved_at, None);
        assert_eq!(d.messages.len(), 2);
        assert_eq!(d.messages[0].created_at, parse_date("2026-01-02T00:00:00Z"));
        assert_eq!(d.messages[1].revision, 0);
        assert_eq!(d.messages[1].created_at, None);
        let bare: SubmissionDetail =
            decode(json!({"id": 1, "kind": "new", "status": "open", "messages": "x"})).unwrap();
        assert!(bare.messages.is_empty());
        assert_eq!(bare.revision, 0);
        assert!(decode::<SubmissionDetail>(json!({"id": 1, "kind": "new", "status": "weird"})).is_none());
    }

    fn stats_data() -> Value {
        json!({
            "downloads": {"total": 700, "series": [
                {"taken_on": "2026-01-01", "downloads": 10, "subscribers": 2, "daily_downloads": null, "daily_subscriber_change": null},
                {"taken_on": "2026-01-02", "downloads": 15.0, "subscribers": 3, "daily_downloads": 5, "daily_subscriber_change": 1}
            ]},
            "ratings": {"likes": 0, "dislikes": 0, "total_votes": 0, "like_percentage": null, "deleted_accounts": 0},
            "subscribers": {"current": 17, "series": []},
            "versions": {"buckets": {"latest": 3, "one_behind": 1, "two_behind": 0, "unavailable": 2, "future": 9},
                "share": [{"release_id": 1, "tag_name": "v1", "subscribers": 3, "share": 0.5, "lag_bucket": "latest"}]}
        })
    }

    #[test]
    fn stats_ok_and_strictness() {
        let s: AddonDeveloperStats = decode(stats_data()).unwrap();
        assert_eq!(s.downloads.total, 700);
        assert_eq!(s.downloads.series[0].taken_on, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(s.downloads.series[0].daily_downloads, None);
        assert_eq!(s.downloads.series[1].downloads, 15);
        assert_eq!(s.downloads.series[1].daily_downloads, Some(5));
        assert_eq!(s.ratings.like_percentage, None);
        assert_eq!(s.subscribers.current, 17);
        assert_eq!(s.versions.buckets.count(VersionLagBucket::Unavailable), 2);
        assert_eq!(s.versions.share[0].lag_bucket, VersionLagBucket::Latest);
        let mut d = stats_data();
        d["ratings"] =
            json!({"likes": 1, "dislikes": 1, "total_votes": 2, "like_percentage": 0.5, "deleted_accounts": 0});
        assert_eq!(decode::<AddonDeveloperStats>(d).unwrap().ratings.like_percentage, Some(0.5));

        let rejects = |edit: &dyn Fn(&mut Value)| {
            let mut d = stats_data();
            edit(&mut d);
            decode::<AddonDeveloperStats>(d).is_none()
        };
        // like_percentage null with votes, a number with zero votes, or no key at all
        assert!(rejects(&|d| d["ratings"]["total_votes"] = json!(2)));
        assert!(rejects(&|d| d["ratings"]["like_percentage"] = json!(0.5)));
        assert!(rejects(&|d| {
            d["ratings"].as_object_mut().unwrap().remove("like_percentage");
        }));
        // a daily figure may be null, but its key is required
        assert!(rejects(&|d| {
            d["downloads"]["series"][0].as_object_mut().unwrap().remove("daily_downloads");
        }));
        assert!(rejects(&|d| d["downloads"]["series"][0]["taken_on"] = json!("2026-1-1")));
        assert!(rejects(&|d| d["downloads"]["series"][0]["taken_on"] = json!("2026-01-01x")));
        assert!(rejects(&|d| d["downloads"]["series"][0]["taken_on"] = json!("2026-13-45")));
        assert!(rejects(&|d| {
            d["versions"]["buckets"].as_object_mut().unwrap().remove("latest");
        }));
        assert!(rejects(&|d| d["versions"]["share"][0]["lag_bucket"] = json!("ancient")));
        assert!(rejects(&|d| d["downloads"]["total"] = json!(1.5)));
        assert!(rejects(&|d| d["downloads"]["total"] = json!("700")));
    }

    #[test]
    fn uploaded_icon_needs_asset_id_and_url() {
        let ok =
            json!({"asset_id": "2f57dce0-67d4-49c4-8af4-e108fb250f1e", "url": "https://assets.example/icons/x.png"});
        assert_eq!(
            decode(ok),
            Some(UploadedIcon {
                asset_id: "2f57dce0-67d4-49c4-8af4-e108fb250f1e".into(),
                url: "https://assets.example/icons/x.png".into(),
            })
        );
        assert_eq!(decode::<UploadedIcon>(json!({"asset_id": "", "url": "u"})), None);
        assert_eq!(decode::<UploadedIcon>(json!({"asset_id": "a"})), None);
        assert_eq!(decode::<UploadedIcon>(Value::Null), None);
    }

    #[test]
    fn dates() {
        let utc = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        assert_eq!(parse_date("2026-01-02T03:04:05Z"), Some(utc("2026-01-02T03:04:05Z")));
        assert_eq!(parse_date("2026-01-02T03:04:05.123+02:00"), Some(utc("2026-01-02T01:04:05.123Z")));
        assert_eq!(parse_date("2026-01-02 03:04:05"), Some(utc("2026-01-02T03:04:05Z")));
        assert_eq!(parse_date("2026-01-02"), Some(utc("2026-01-02T00:00:00Z")));
        assert_eq!(parse_date("garbage"), None);
        assert_eq!(parse_date(""), None);
    }
}
