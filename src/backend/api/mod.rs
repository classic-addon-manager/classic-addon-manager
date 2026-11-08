//! Typed endpoints of the server. Every call goes through [`client`], developer endpoints live in
//! `ui::pages::developer::api` on the same client.
mod client;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use client::{
    API_URL, ApiError, Body, FieldError, FormPart, Method, Stream, client_header, download, open_stream, request,
    request_bare,
};

use super::serde_util::{go_time, null_default};
use super::shared::AddonManifest;

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
}
