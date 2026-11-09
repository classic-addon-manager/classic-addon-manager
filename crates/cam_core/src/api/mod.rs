//! Typed endpoints of the server and their wire types. Every call goes through [`client`]: the addon catalog and
//! releases here, the developer endpoints in [`developer`] and the AI chat stream in [`chat`].
pub mod chat;
mod client;
pub mod developer;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use client::{
    API_URL, ApiError, Body, FieldError, FormPart, Method, Stream, client_header, download, open_stream, request,
    request_bare,
};

use crate::serde_util::{go_time, null_default};

/// What [`encode_uri_component`] escapes: every byte except ASCII letters, digits and `-_.!~*'()`.
const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// Percent-encodes `s` with upper-case hex so it fits into one URL path segment or query value. A `/` or a space is
/// escaped too (`v1.0/beta 1` becomes `v1.0%2Fbeta%201`).
pub fn encode_uri_component(s: &str) -> String {
    utf8_percent_encode(s, URI_COMPONENT).to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AddonManifest {
    #[serde(deserialize_with = "null_default")]
    pub name: String,
    #[serde(deserialize_with = "null_default")]
    pub alias: String,
    #[serde(deserialize_with = "null_default")]
    pub dependencies: Vec<String>,
    #[serde(deserialize_with = "null_default")]
    pub description: String,
    #[serde(deserialize_with = "null_default")]
    pub author: String,
    #[serde(deserialize_with = "null_default")]
    pub repo: String,
    #[serde(deserialize_with = "null_default")]
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(deserialize_with = "null_default")]
    pub tags: Vec<String>,
    pub downloads: i64,
    #[serde(rename = "like_percentage")]
    pub like_percent: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kofi: Option<String>,
    #[serde(with = "go_time")]
    pub added_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub library: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Tag {
    #[serde(rename = "ref")]
    pub r#ref: String,
    pub sha: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Release {
    pub zipball_url: String,
    pub tag_name: String,
    pub body: String,
    #[serde(with = "go_time")]
    pub published_at: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "null_default")]
    pub tag: Tag,
    /// Lowercase hex SHA-256 of the stored release archive.
    #[serde(deserialize_with = "null_default")]
    pub sha256: String,
    /// Size of the stored release archive in bytes.
    #[serde(deserialize_with = "null_default")]
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ApplicationRelease {
    pub version: String,
    pub url: String,
    pub checksum: String,
}

/// The signed-in account (`GET /me`).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct User {
    pub username: String,
    pub avatar: String,
    pub discord_id: String,
}

/// `release` + `tag` pair the release endpoints return, the tag is folded into the release.
#[derive(Default, Deserialize)]
struct ReleaseWithTag {
    #[serde(default, deserialize_with = "null_default")]
    release: Release,
    #[serde(default, deserialize_with = "null_default")]
    tag: Tag,
}

impl ReleaseWithTag {
    fn into_release(self) -> Release {
        Release { tag: self.tag, ..self.release }
    }
}

fn missing_fields(what: &str) -> ApiError {
    ApiError::Decode(format!("{what} is missing required fields"))
}

/// The addon catalog (`GET /addons`).
pub fn catalog() -> Result<Vec<AddonManifest>, ApiError> {
    let manifests: Option<Vec<AddonManifest>> = request_bare(Method::Get, "/addons", Body::None)?;
    let manifests = manifests.unwrap_or_default();
    log::info!("Retrieved {} addon manifests from remote source", manifests.len());
    Ok(manifests)
}

/// One release of an addon, `version` is a tag or `latest`.
pub fn release(name: &str, version: &str) -> Result<Release, ApiError> {
    let data: ReleaseWithTag = request(Method::Get, &format!("/addon/{name}/release/{version}"), Body::None)?;
    let release = data.into_release();
    if release.zipball_url.is_empty() || release.tag_name.is_empty() || release.published_at.is_none() {
        return Err(missing_fields("release"));
    }
    Ok(release)
}

/// Every downloadable release of an addon, in server order.
pub fn releases(name: &str) -> Result<Vec<Release>, ApiError> {
    request(Method::Get, &format!("/addon/{name}/releases"), Body::None)
}

/// The latest release of several addons in one call. Addons without a release are missing from the map.
pub fn latest_releases(names: &[String]) -> Result<HashMap<String, Release>, ApiError> {
    let data: HashMap<String, ReleaseWithTag> =
        request(Method::Post, "/latest_releases", Body::Json(json!({ "addons": names })))?;
    Ok(data.into_iter().map(|(name, r)| (name, r.into_release())).collect())
}

/// The newest app release for this OS.
pub fn latest_application_release() -> Result<ApplicationRelease, ApiError> {
    let release: ApplicationRelease = request(Method::Get, "/latest_application_release", Body::None)?;
    if release.version.is_empty() || release.url.is_empty() {
        return Err(missing_fields("application release"));
    }
    Ok(release)
}

pub fn me() -> Result<User, ApiError> {
    request_bare(Method::Get, "/me", Body::None)
}

/// Catalog entries of the addons the signed-in user is subscribed to.
pub fn subscribed_addons() -> Result<Vec<AddonManifest>, ApiError> {
    request(Method::Get, "/me/addons", Body::None)
}

pub fn unsubscribe(name: &str) -> Result<(), ApiError> {
    request(Method::Post, &format!("/addon/{name}/unsubscribe"), Body::None)
}

/// The signed-in user's vote on an addon: 1 like, -1 dislike, 0 none.
pub fn my_rating(name: &str) -> Result<i8, ApiError> {
    #[derive(Default, Deserialize)]
    struct Rating {
        rating: i64,
    }
    let data: Rating = request(Method::Get, &format!("/addon/{name}/my-rating"), Body::None)?;
    match data.rating {
        r @ -1..=1 => Ok(r as i8),
        r => Err(ApiError::Decode(format!("rating {r} is out of range"))),
    }
}

pub fn rate(name: &str, like: bool) -> Result<(), ApiError> {
    request::<serde_json::Value>(Method::Post, &format!("/addon/{name}/rate"), Body::Json(json!({ "is_like": like })))?;
    Ok(())
}

/// An addon's `README.md` from GitHub, `None` when the repository has none on that branch (empty = `main`).
pub fn readme(repo: &str, branch: &str) -> Result<Option<String>, ApiError> {
    let branch = if branch.is_empty() { "main" } else { branch };
    client::get_external_text(&format!("https://raw.githubusercontent.com/{repo}/refs/heads/{branch}/README.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_missing_published_at_is_none() {
        let r: Release = serde_json::from_str(r#"{"zipball_url":"u","tag_name":"v1"}"#).unwrap();
        assert_eq!(r.published_at, None);
    }

    #[test]
    fn manifest_decodes_nulls_and_optionals() {
        let m: AddonManifest = serde_json::from_str(
            r#"{"name":"a","dependencies":null,"tags":null,"like_percentage":null,"added_at":"2024-01-02T03:04:05Z"}"#,
        )
        .unwrap();
        assert!(m.dependencies.is_empty());
        assert_eq!(m.like_percent, None);
        let out = serde_json::to_value(&m).unwrap();
        assert!(out.get("icon").is_none());
        assert_eq!(out["like_percentage"], serde_json::Value::Null);
    }

    #[test]
    fn catalog_with_null_branch_and_strings_still_parses() {
        // Documented `GET /addons` example, one entry with `branch: null` and null string fields.
        let body = r#"[
          {"name":"OpenAddon","alias":"Open Addon","dependencies":[],"author":"dev","description":"Example addon.",
           "repo":"example/OpenAddon","branch":null,"tags":["UI"],"downloads":42,"like_percentage":100,"votes":3,
           "library":false,"added_at":"2026-09-20T10:00:00Z"},
          {"name":"Other","alias":null,"dependencies":null,"author":null,"description":null,"repo":null,
           "branch":"main","tags":null,"downloads":1,"like_percentage":null,"votes":0,"library":true,
           "added_at":"2026-09-21T10:00:00Z"}
        ]"#;
        let list: Option<Vec<AddonManifest>> = serde_json::from_str(body).unwrap();
        let list = list.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "OpenAddon");
        assert_eq!(list[0].branch, "");
        assert_eq!(list[0].repo, "example/OpenAddon");
        assert_eq!(list[1].branch, "main");
        assert_eq!((list[1].alias.as_str(), list[1].author.as_str(), list[1].repo.as_str()), ("", "", ""));
    }

    #[test]
    fn uri_component_encoding() {
        assert_eq!(encode_uri_component("v1.0/beta 1"), "v1.0%2Fbeta%201");
        assert_eq!(encode_uri_component("2026-10-03"), "2026-10-03");
        assert_eq!(encode_uri_component("a-_.!~*'()z"), "a-_.!~*'()z");
        assert_eq!(encode_uri_component("é+&"), "%C3%A9%2B%26");
    }
}
