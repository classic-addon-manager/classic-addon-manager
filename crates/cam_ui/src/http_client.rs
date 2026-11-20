//! `gpui_kit::http_client::HttpClient` backed by `ureq`, so `gpui_kit::img("https://...")` (README images, badges, avatars) works.
//!
//! gpui's default client is a null client (nothing in gpui-kit replaces it on native). [`install`] is idempotent (it
//! sets the client once, behind a marker global) and is called by the markdown widget on first render and by the
//! app at startup. Requests run on a short-lived thread per request (never on gpui's executors).
//!
//! With a [`DiskCache`] ([`install_with_cache`]), GETs of the URLs it holds are answered from disk when stored, and a
//! fetched response is stored after it was handed over when it is complete, uncompressed, not redirected and
//! declared `immutable` by the server.
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use gpui_kit::http_client::http::HeaderValue;
use gpui_kit::http_client::{AsyncBody, HttpClient, Method, Request, Response, Url};
use gpui_kit::{App, Global};
use ureq::ResponseExt as _;
use ureq::http::HeaderMap;
use ureq::http::header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HeaderName};

use crate::http_cache::{DiskCache, MAX_BODY};

const USER_AGENT: &str = concat!("ClassicAddonManager/", env!("CARGO_PKG_VERSION"));

pub struct UreqHttpClient {
    agent: ureq::Agent,
    user_agent: HeaderValue,
    cache: Option<Arc<DiskCache>>,
}

impl UreqHttpClient {
    pub fn new(cache: Option<Arc<DiskCache>>) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(USER_AGENT)
            .build()
            .into();
        Self { agent, user_agent: HeaderValue::from_static(USER_AGENT), cache }
    }

    /// Status and body of `uri`, and whether the response may be stored (only checked when `storable` is asked for).
    fn fetch(agent: &ureq::Agent, uri: &str, storable: bool) -> anyhow::Result<(u16, Bytes, bool)> {
        let mut resp = agent.get(uri).call()?;
        let status = resp.status().as_u16();
        let expected_len = if storable { storable_len(status, resp.get_uri(), uri, resp.headers()) } else { None };
        let body = resp.body_mut().read_to_vec()?;
        let storable = expected_len.is_some_and(|len| len == body.len() as u64);
        Ok((status, Bytes::from(body), storable))
    }
}

/// The `Content-Length` of a response worth keeping: a complete 200 with an uncompressed image, served for `uri`
/// itself (`final_uri`, after redirects) and declared `immutable` without `no-store`, `no-cache` or `private`.
/// Anything unusual (a quoted directive, repeated or unreadable headers) is simply not stored.
fn storable_len(status: u16, final_uri: &ureq::http::Uri, uri: &str, headers: &HeaderMap) -> Option<u64> {
    if status != 200 || *final_uri != *uri {
        return None;
    }
    let mut immutable = false;
    for value in headers.get_all(CACHE_CONTROL) {
        let value = value.to_str().ok().filter(|v| !v.contains('"'))?;
        for directive in value.split(',') {
            let name = directive.split('=').next().unwrap_or_default().trim();
            if name.eq_ignore_ascii_case("immutable") {
                immutable = true;
            } else if ["no-store", "no-cache", "private"].iter().any(|n| name.eq_ignore_ascii_case(n)) {
                return None;
            }
        }
    }
    let identity = headers
        .get_all(CONTENT_ENCODING)
        .iter()
        .all(|v| v.to_str().is_ok_and(|e| e.trim().eq_ignore_ascii_case("identity")));
    let image = single(headers, CONTENT_TYPE)?.starts_with("image/");
    (immutable && identity && image)
        .then(|| single(headers, CONTENT_LENGTH)?.trim().parse().ok())
        .flatten()
        .filter(|&len| len <= MAX_BODY)
}

/// The value of a header that appears exactly once.
fn single(headers: &HeaderMap, name: HeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    values.next().is_none().then(|| value.to_str().ok()).flatten()
}

fn response(status: u16, body: Bytes) -> anyhow::Result<Response<AsyncBody>> {
    Ok(Response::builder().status(status).body(AsyncBody::from_bytes(body))?)
}

impl HttpClient for UreqHttpClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        Some(&self.user_agent)
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(
        &self,
        req: Request<AsyncBody>,
    ) -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<Response<AsyncBody>>> + Send + 'static>> {
        let agent = self.agent.clone();
        let uri = req.uri().to_string();
        let cache = self.cache.clone().filter(|_| req.method() == Method::GET);
        Box::pin(async move {
            let (tx, rx) = async_channel::bounded(1);
            std::thread::spawn(move || {
                let record = cache.as_deref().and_then(|cache| Some((cache, cache.path(&uri)?)));
                if let Some((cache, path)) = &record
                    && let Some(body) = cache.read(path)
                {
                    let _ = tx.send_blocking(response(200, body));
                    return;
                }
                match Self::fetch(&agent, &uri, record.is_some()) {
                    Ok((status, body, true)) => {
                        let _ = tx.send_blocking(response(status, body.clone()));
                        if let Some((cache, path)) = record {
                            cache.store(&path, &body);
                        }
                    }
                    Ok((status, body, false)) => {
                        let _ = tx.send_blocking(response(status, body));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(Err(e));
                    }
                }
            });
            rx.recv().await?
        })
    }
}

struct Installed;
impl Global for Installed {}

/// Installs the ureq client as gpui's HTTP client (once).
pub fn install(cx: &mut App) {
    install_with_cache(cx, None);
}

/// Like [`install`], with `cache` for the responses it holds. Only the first install takes effect, so the app calls
/// this before anything else installs the client.
pub fn install_with_cache(cx: &mut App, cache: Option<Arc<DiskCache>>) {
    if cx.has_global::<Installed>() {
        return;
    }
    cx.set_global(Installed);
    cx.set_http_client(Arc::new(UreqHttpClient::new(cache)));
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: &str = "https://assets.gaijin.dev/icons/23f289ab-6aaf-495a-8bf4-0ed953bcb6c7.png";

    /// The headers the CDN sends for an icon.
    fn cdn_headers() -> Vec<(HeaderName, &'static str)> {
        vec![
            (CONTENT_TYPE, "image/png"),
            (CONTENT_LENGTH, "103107"),
            (CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ]
    }

    fn admitted(status: u16, final_uri: &str, headers: &[(HeaderName, &'static str)]) -> Option<u64> {
        let map = headers.iter().map(|(name, value)| (name.clone(), HeaderValue::from_static(value))).collect();
        storable_len(status, &final_uri.parse().unwrap(), ICON, &map)
    }

    #[test]
    fn only_complete_immutable_images_are_stored() {
        assert_eq!(admitted(200, ICON, &cdn_headers()), Some(103107));

        assert_eq!(admitted(206, ICON, &cdn_headers()), None);
        assert_eq!(admitted(200, "https://assets.gaijin.dev/elsewhere.png", &cdn_headers()), None);
        let with = |extra: (HeaderName, &'static str)| [cdn_headers(), vec![extra]].concat();
        for extra in [
            (CACHE_CONTROL, "no-store"),
            (CACHE_CONTROL, "private=\"Set-Cookie\""),
            (CONTENT_ENCODING, "gzip"),
            (CONTENT_LENGTH, "103107"),
        ] {
            assert_eq!(admitted(200, ICON, &with(extra.clone())), None, "{extra:?}");
        }
        let without = |name: HeaderName| cdn_headers().into_iter().filter(|(n, _)| *n != name).collect::<Vec<_>>();
        for name in [CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE] {
            assert_eq!(admitted(200, ICON, &without(name.clone())), None, "without {name}");
        }
    }
}
