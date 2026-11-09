//! `gpui_kit::http_client::HttpClient` backed by `ureq`, so `gpui_kit::img("https://...")` (README images, badges, avatars) works.
//!
//! gpui's default client is a null client (nothing in gpui-kit replaces it on native). [`install`] is idempotent (it
//! sets the client once, behind a marker global) and is called by the markdown widget on first render and by the
//! app at startup. Requests run on a short-lived thread per request (never on gpui's executors).
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::http_client::http::HeaderValue;
use gpui_kit::http_client::{AsyncBody, HttpClient, Request, Response, Url};
use gpui_kit::{App, Global};

const USER_AGENT: &str = concat!("ClassicAddonManager/", env!("CARGO_PKG_VERSION"));

pub struct UreqHttpClient {
    agent: ureq::Agent,
    user_agent: HeaderValue,
}

impl UreqHttpClient {
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(USER_AGENT)
            .build()
            .into();
        Self { agent, user_agent: HeaderValue::from_static(USER_AGENT) }
    }

    fn fetch(agent: &ureq::Agent, uri: &str) -> anyhow::Result<Response<AsyncBody>> {
        let mut resp = agent.get(uri).call()?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_vec()?;
        Ok(Response::builder().status(status).body(AsyncBody::from(body))?)
    }
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
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
        Box::pin(async move {
            let (tx, rx) = async_channel::bounded(1);
            std::thread::spawn(move || {
                let _ = tx.send_blocking(Self::fetch(&agent, &uri));
            });
            rx.recv().await?
        })
    }
}

struct Installed;
impl Global for Installed {}

/// Installs the ureq client as gpui's HTTP client (once).
pub fn install(cx: &mut App) {
    if cx.has_global::<Installed>() {
        return;
    }
    cx.set_global(Installed);
    cx.set_http_client(Arc::new(UreqHttpClient::new()));
}
