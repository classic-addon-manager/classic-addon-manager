//! Transport for the server at [`API_URL`]: one header set, one request path, and [`ApiError`] for every failure.
use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::sync::LazyLock;
use std::time::Duration;

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use serde_json::value::RawValue;

use crate::auth;

pub const API_URL: &str = "https://aac.gaijin.dev";

const JSON: &str = "application/json";

/// Why a request failed. Callers branch on the variant, never on the server's wording.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// No usable response: DNS, connection, TLS, timeout, or the body broke off.
    Network(String),
    /// 401: not signed in, or the session token was rejected.
    Unauthorized,
    /// 404.
    NotFound,
    /// 409 naming the record the request conflicts with.
    Conflict { id: i64, message: String },
    /// 400 with per-field validation errors.
    Validation(Vec<FieldError>),
    /// Any other failure the server reported: a non-success status, or `"status": false` in the envelope.
    Rejected { status: u16, message: String },
    /// The response did not have the expected shape.
    Decode(String),
}

impl ApiError {
    /// The server's explanation, when it sent a non-empty one.
    pub fn server_message(&self) -> Option<&str> {
        match self {
            ApiError::Conflict { message, .. } | ApiError::Rejected { message, .. } if !message.is_empty() => {
                Some(message)
            }
            _ => None,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(e) => write!(f, "network error: {e}"),
            ApiError::Unauthorized => f.write_str("not signed in"),
            ApiError::NotFound => f.write_str("not found"),
            ApiError::Conflict { id, message } => write!(f, "{message} (id {id})"),
            ApiError::Validation(errors) => {
                f.write_str("invalid input")?;
                for (i, e) in errors.iter().enumerate() {
                    write!(f, "{} {}: {}", if i == 0 { ":" } else { ";" }, e.field, e.message)?;
                }
                Ok(())
            }
            ApiError::Rejected { status, message } if message.is_empty() => {
                write!(f, "server responded with HTTP {status}")
            }
            ApiError::Rejected { status, message } => write!(f, "{message} (HTTP {status})"),
            ApiError::Decode(e) => write!(f, "unexpected server response: {e}"),
        }
    }
}

impl std::error::Error for ApiError {}

/// One entry of a validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
}

pub enum Body {
    None,
    Json(Value),
    Form(Vec<FormPart>),
}

/// One multipart/form-data part.
pub struct FormPart {
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

fn agent(timeout: Duration, global: bool, max_redirects: u32) -> ureq::Agent {
    let config = ureq::Agent::config_builder().http_status_as_error(false).max_redirects(max_redirects);
    let config = if global { config.timeout_global(Some(timeout)) } else { config.timeout_connect(Some(timeout)) };
    ureq::Agent::new_with_config(config.build())
}

static REQUEST_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| agent(Duration::from_secs(30), true, 10));
/// No overall timeout: a stream stays open as long as the server sends.
static STREAM_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| agent(Duration::from_secs(30), false, 10));
/// Doesn't follow redirects itself: [`download`] does, so each hop gets the headers its host may see.
static DOWNLOAD_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| agent(DOWNLOAD_TIMEOUT, true, 0));

/// `X-Client` value. The server counts launches only when it is exactly this prefix followed by a semver version.
pub fn client_header() -> String {
    format!("Classic Addon Manager {}", crate::VERSION)
}

/// The header set every request to the server carries. The token is sent whenever there is one: public
/// endpoints ignore it, and user endpoints answer 401 without it.
fn with_app_headers<B>(req: ureq::RequestBuilder<B>, accept: &str) -> ureq::RequestBuilder<B> {
    app_headers(req, accept, &auth::get_token())
}

fn app_headers<B>(mut req: ureq::RequestBuilder<B>, accept: &str, token: &str) -> ureq::RequestBuilder<B> {
    req = req.header("X-Client", client_header()).header("X-Client-OS", std::env::consts::OS).header("Accept", accept);
    if !token.is_empty() {
        req = req.header("X-Token", token);
    }
    req
}

fn network(e: impl fmt::Display) -> ApiError {
    ApiError::Network(e.to_string())
}

fn read_text(resp: &mut ureq::http::Response<ureq::Body>) -> Result<String, ApiError> {
    resp.body_mut().with_config().limit(256 * 1024 * 1024).lossy_utf8(true).read_to_string().map_err(network)
}

/// Unique enough per request: the boundary only has to differ from the part contents.
fn form_boundary() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(std::process::id() as u64);
    format!("----CAMBoundary{:032x}", h.finish() as u128 * 0x1_0000_0001)
}

fn quote_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\r', "%0D").replace('\n', "%0A")
}

fn encode_form(parts: &[FormPart], boundary: &str) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                quote_escape(&part.name),
                quote_escape(&part.filename),
                part.content_type
            )
            .as_bytes(),
        );
        body.extend_from_slice(&part.data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// Sends one request to the server and returns the status and body text. POST and PUT always carry a
/// `Content-Type`, even without a body.
fn send(method: Method, path: &str, body: Body) -> Result<(u16, String), ApiError> {
    let url = format!("{API_URL}{path}");
    let a = &*REQUEST_AGENT;
    let resp = match method {
        Method::Get => with_app_headers(a.get(&url), JSON).call(),
        Method::Post | Method::Put => {
            let req = if method == Method::Post { a.post(&url) } else { a.put(&url) };
            let req = with_app_headers(req, JSON);
            match body {
                Body::None => req.header("Content-Type", JSON).send(&[][..]),
                Body::Json(value) => req.header("Content-Type", JSON).send(value.to_string().as_bytes()),
                Body::Form(parts) => {
                    let boundary = form_boundary();
                    let bytes = encode_form(&parts, &boundary);
                    req.header("Content-Type", format!("multipart/form-data; boundary={boundary}")).send(&bytes[..])
                }
            }
        }
    };
    let mut resp = resp.map_err(network)?;
    let status = resp.status().as_u16();
    Ok((status, read_text(&mut resp)?))
}

fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

/// Field errors of a failed validation: either `{"valid": false, "errors": [{field, message}, ...]}` (entries of the
/// wrong shape are skipped) or a single `{field, message}` (icon upload).
fn field_errors(data: &Value) -> Option<Vec<FieldError>> {
    if let Ok(single) = FieldError::deserialize(data) {
        return Some(vec![single]);
    }
    if data.get("valid").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    let errors = data.get("errors")?.as_array()?;
    Some(errors.iter().filter_map(|e| FieldError::deserialize(e).ok()).collect())
}

/// Maps a non-success response to its error. Error bodies are `{"status": false, "message": ..., "data": ...}`;
/// a body that isn't JSON (a proxy error page) still maps by status.
fn classify(status: u16, body: &str) -> ApiError {
    let body: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let message = body.get("message").and_then(Value::as_str).unwrap_or_default().to_owned();
    let data = body.get("data").unwrap_or(&Value::Null);
    match status {
        401 => ApiError::Unauthorized,
        404 => ApiError::NotFound,
        409 => match data.get("id").and_then(Value::as_i64) {
            Some(id) => ApiError::Conflict { id, message },
            None => ApiError::Rejected { status, message },
        },
        400 => match field_errors(data) {
            Some(errors) => ApiError::Validation(errors),
            None => ApiError::Rejected { status, message },
        },
        _ => ApiError::Rejected { status, message },
    }
}

/// Decodes the `data` of a `{status, message, data}` envelope. A missing or `null` `data` decodes as `T::default()`.
/// `status` is checked before `data` is decoded, so a failure keeps the server's message whatever its `data` holds.
fn decode_envelope<T: DeserializeOwned + Default>(status: u16, body: &str) -> Result<T, ApiError> {
    if !is_success(status) {
        return Err(classify(status, body));
    }
    #[derive(Deserialize)]
    struct Envelope {
        status: bool,
        #[serde(default)]
        message: String,
        data: Option<Value>,
    }
    let envelope: Envelope = serde_json::from_str(body).map_err(|e| ApiError::Decode(e.to_string()))?;
    if !envelope.status {
        return Err(ApiError::Rejected { status, message: envelope.message });
    }
    match envelope.data {
        None => Ok(T::default()),
        Some(data) => serde_json::from_value(data).map_err(|e| ApiError::Decode(e.to_string())),
    }
}

/// [`decode_envelope`] for large replies: [`decode_envelope_direct`] builds `T` straight from the text of `data`, and
/// whenever it can't vouch for its outcome [`decode_envelope`] decides, so every input gets the same result and error
/// text. Small replies don't gain from it: the extra checking pass costs more than the tree it saves.
///
/// The guarantee holds for structs, string-keyed maps and integers up to 64 bits. A `T` that keeps raw JSON text
/// (`RawValue`), reads 128-bit integers or sees duplicate map keys in order (two keys aliased to one enum variant)
/// could get a different result, because the tree drops whitespace, has no 128-bit numbers and keeps one value per key.
fn decode_large_envelope<T: DeserializeOwned + Default>(status: u16, body: &str) -> Result<T, ApiError> {
    if is_success(status)
        && let Some(result) = decode_envelope_direct(status, body)
    {
        return result;
    }
    decode_envelope(status, body)
}

/// [`decode_envelope`] without the intermediate tree. `None` where the outcome could differ: a body or `data` the tree
/// parse rejects, and every failure to build `T`, which covers the inputs the two read differently (such as duplicate
/// keys) and leaves error texts to the tree decoder.
fn decode_envelope_direct<T: DeserializeOwned + Default>(status: u16, body: &str) -> Option<Result<T, ApiError>> {
    #[derive(Deserialize)]
    struct Envelope<'a> {
        status: bool,
        #[serde(default)]
        message: String,
        #[serde(borrow)]
        data: Option<&'a RawValue>,
    }
    let envelope: Envelope = serde_json::from_str(body).ok()?;
    if let Some(data) = envelope.data {
        // Capturing `data` only checks its syntax; the tree parse also rejects out-of-range numbers, for example.
        ValueCheck { level: 1 }.deserialize(&mut serde_json::Deserializer::from_str(data.get())).ok()?;
    }
    if !envelope.status {
        return Some(Err(ApiError::Rejected { status, message: envelope.message }));
    }
    match envelope.data {
        None => Some(Ok(T::default())),
        Some(data) => serde_json::from_str(data.get()).ok().map(Ok),
    }
}

/// Walks a JSON value like parsing it into a [`Value`] does, without building anything, and fails wherever that
/// parse fails (and on a few inputs it accepts). `level` is the number of containers open around the value in the
/// whole body, for serde_json's nesting limit.
#[derive(Clone, Copy)]
struct ValueCheck {
    level: u8,
}

impl ValueCheck {
    /// serde_json rejects the 128th level of nesting.
    const MAX_LEVEL: u8 = 127;

    fn enter<E: de::Error>(self) -> Result<Self, E> {
        match self.level {
            Self::MAX_LEVEL.. => Err(E::custom("nested too deeply")),
            level => Ok(Self { level: level + 1 }),
        }
    }
}

impl<'de> DeserializeSeed<'de> for ValueCheck {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ValueCheck {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let item = self.enter()?;
        while seq.next_element_seed(item)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let value = self.enter()?;
        while map.next_key_seed(KeyCheck)?.is_some() {
            map.next_value_seed(value)?;
        }
        Ok(())
    }
}

/// An object key for [`ValueCheck`]. Parsing into a [`Value`] reads a first key equal to one of serde_json's private
/// tokens as a marker (a raw value or an arbitrary-precision number), so such keys fail the check.
struct KeyCheck;

impl<'de> DeserializeSeed<'de> for KeyCheck {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for KeyCheck {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an object key")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<(), E> {
        match key.starts_with("$serde_json::private::") {
            true => Err(E::custom("reserved key")),
            false => Ok(()),
        }
    }
}

/// Sends a request and decodes the `data` of the `{status, message, data}` envelope the server wraps responses in.
pub fn request<T: DeserializeOwned + Default>(method: Method, path: &str, body: Body) -> Result<T, ApiError> {
    let (status, text) = send(method, path, body)?;
    decode_envelope(status, &text)
}

/// [`request`] for replies large enough that skipping the intermediate tree pays off, see [`decode_large_envelope`]
/// for the types it is exact for.
pub(crate) fn request_large<T: DeserializeOwned + Default>(
    method: Method,
    path: &str,
    body: Body,
) -> Result<T, ApiError> {
    let (status, text) = send(method, path, body)?;
    decode_large_envelope(status, &text)
}

/// Like [`request`], for the endpoints that answer with bare JSON instead of an envelope (`/me`, `/addons`).
pub fn request_bare<T: DeserializeOwned>(method: Method, path: &str, body: Body) -> Result<T, ApiError> {
    let (status, text) = send(method, path, body)?;
    if !is_success(status) {
        return Err(classify(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| ApiError::Decode(e.to_string()))
}

/// Line reader over a streaming response body (server-sent events).
pub struct Stream {
    reader: BufReader<Box<dyn Read + Send>>,
}

impl Iterator for Stream {
    type Item = std::io::Result<String>;

    /// The next line without its trailing `\r?\n`, `None` at the end of the stream.
    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                while line.ends_with('\n') || line.ends_with('\r') {
                    line.pop();
                }
                Some(Ok(line))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// GETs `path` as an event stream. Only connecting is time-limited.
pub fn open_stream(path: &str) -> Result<Stream, ApiError> {
    let url = format!("{API_URL}{path}");
    let mut resp = with_app_headers(STREAM_AGENT.get(&url), "text/event-stream").call().map_err(network)?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(classify(status, &read_text(&mut resp)?));
    }
    let reader: Box<dyn Read + Send> = Box::new(resp.into_body().into_reader());
    Ok(Stream { reader: BufReader::new(reader) })
}

/// Redirects a download follows before giving up.
const MAX_DOWNLOAD_REDIRECTS: usize = 10;
/// Time limit for a whole download, redirects and body included.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Whether `url` is on the server at `api_url`, and so may carry the app headers and the session token.
fn is_api_url(url: &str, api_url: &str) -> bool {
    url.strip_prefix(api_url).is_some_and(|rest| rest.starts_with('/'))
}

/// GETs a file (10 minutes for the whole download). Redirects are followed here, one request per hop, so that only
/// requests to the server carry the app headers: addon downloads redirect to the asset host, which must not see the
/// session token.
pub fn download(url: &str) -> Result<impl Read + Send + use<>, ApiError> {
    download_from(url, API_URL, &auth::get_token())
}

/// [`download`] against the server at `api_url`, sending `token` to it.
fn download_from(url: &str, api_url: &str, token: &str) -> Result<impl Read + Send + use<>, ApiError> {
    let accept = "application/octet-stream, */*";
    let deadline = std::time::Instant::now() + DOWNLOAD_TIMEOUT;
    let mut url = url::Url::parse(url).map_err(|e| ApiError::Network(format!("invalid URL {url}: {e}")))?;
    for _ in 0..=MAX_DOWNLOAD_REDIRECTS {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(ApiError::Network("download timed out".into()));
        }
        // Each hop gets what is left of the overall limit, which then also bounds reading the body.
        let req = DOWNLOAD_AGENT.get(url.as_str()).config().timeout_global(Some(remaining)).build();
        let req = if is_api_url(url.as_str(), api_url) {
            app_headers(req, accept, token)
        } else {
            req.header("Accept", accept)
        };
        let mut resp = req.call().map_err(network)?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp.into_body().into_reader());
        }
        let location = resp.headers().get("Location").and_then(|l| l.to_str().ok());
        match location {
            Some(location) if status.is_redirection() => {
                url = url.join(location).map_err(|e| ApiError::Network(format!("invalid redirect {location}: {e}")))?;
            }
            _ => return Err(classify(status.as_u16(), &read_text(&mut resp)?)),
        }
    }
    Err(ApiError::Network(format!("more than {MAX_DOWNLOAD_REDIRECTS} redirects")))
}

/// GETs a text file from another host, without app headers. `Ok(None)` for a non-success status.
pub fn get_external_text(url: &str) -> Result<Option<String>, ApiError> {
    let mut resp = REQUEST_AGENT.get(url).call().map_err(network)?;
    let status = resp.status().as_u16();
    let text = read_text(&mut resp)?;
    Ok(is_success(status).then_some(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_header_has_version() {
        assert_eq!(client_header(), concat!("Classic Addon Manager ", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn error_statuses_map_to_variants() {
        assert_eq!(classify(401, r#"{"status":false,"message":"invalid token"}"#), ApiError::Unauthorized);
        assert_eq!(classify(404, r#"{"status":false,"message":"addon not found"}"#), ApiError::NotFound);
        assert_eq!(
            classify(409, r#"{"status":false,"message":"submission not open","data":{"id":9}}"#),
            ApiError::Conflict { id: 9, message: "submission not open".into() }
        );
        // A conflict without the record id carries no more than the message.
        assert_eq!(
            classify(409, r#"{"status":false,"message":"submission not open","data":{}}"#),
            ApiError::Rejected { status: 409, message: "submission not open".into() }
        );
        assert_eq!(
            classify(502, "<html>Bad gateway</html>"),
            ApiError::Rejected { status: 502, message: String::new() }
        );
    }

    #[test]
    fn validation_keeps_well_formed_entries() {
        let body = r#"{"status":false,"message":"invalid addon","data":{"valid":false,"errors":[
            {"field":"name","message":"too short"},{"field":3,"message":"skipped"}]}}"#;
        assert_eq!(
            classify(400, body),
            ApiError::Validation(vec![FieldError { field: "name".into(), message: "too short".into() }])
        );
        let no_errors = r#"{"status":false,"message":"invalid addon","data":{"valid":false}}"#;
        assert_eq!(classify(400, no_errors), ApiError::Rejected { status: 400, message: "invalid addon".into() });
        let icon =
            r#"{"status":false,"message":"invalid icon","data":{"field":"icon","message":"Icon must be square."}}"#;
        assert_eq!(
            classify(400, icon),
            ApiError::Validation(vec![FieldError { field: "icon".into(), message: "Icon must be square.".into() }])
        );
    }

    #[test]
    fn envelope_status_false_is_rejected_even_on_200() {
        let r = decode_envelope::<Value>(200, r#"{"status":false,"message":"boom","data":null}"#);
        assert_eq!(r, Err(ApiError::Rejected { status: 200, message: "boom".into() }));
        // Error details that don't fit the success type must not turn the rejection into a decode error.
        let r = decode_envelope::<Vec<String>>(200, r#"{"status":false,"message":"busy","data":{"retry":5}}"#);
        assert_eq!(r, Err(ApiError::Rejected { status: 200, message: "busy".into() }));
    }

    #[test]
    fn only_server_urls_get_app_headers() {
        assert!(is_api_url(&format!("{API_URL}/addon/x/download"), API_URL));
        assert!(!is_api_url(API_URL, API_URL));
        assert!(!is_api_url(&format!("{API_URL}.evil.example/addon"), API_URL));
        assert!(!is_api_url(&format!("{API_URL}@evil.example/addon"), API_URL));
        assert!(!is_api_url("https://assets.gaijin.dev/addons/x.zip", API_URL));
    }

    fn listen() -> (std::net::TcpListener, String) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        (listener, base)
    }

    /// Answers one connection per response, in order, and returns each request's head, lowercased.
    fn serve(listener: std::net::TcpListener, responses: Vec<String>) -> std::thread::JoinHandle<Vec<String>> {
        use std::io::Write as _;
        std::thread::spawn(move || {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut head = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                while reader.read_line(&mut head).unwrap() > 2 && !head.ends_with("\r\n\r\n") {}
                requests.push(head.to_ascii_lowercase());
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        })
    }

    fn redirect(location: &str) -> String {
        format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    }

    #[test]
    fn token_goes_only_to_the_server_across_redirects() {
        let (api, api_base) = listen();
        let (assets, assets_base) = listen();
        let api = serve(
            api,
            vec![
                redirect("/files/x.zip"),
                redirect(&format!("{assets_base}/x.zip")),
                "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nzip!".into(),
            ],
        );
        let assets = serve(assets, vec![redirect(&format!("{api_base}/back"))]);

        let mut body = String::new();
        download_from(&format!("{api_base}/addon/x/download"), &api_base, "secret")
            .unwrap()
            .read_to_string(&mut body)
            .unwrap();
        assert_eq!(body, "zip!");

        let api = api.join().unwrap();
        let paths: Vec<_> = api.iter().map(|r| r.lines().next().unwrap_or_default()).collect();
        assert_eq!(paths, ["get /addon/x/download http/1.1", "get /files/x.zip http/1.1", "get /back http/1.1"]);
        for request in &api {
            assert!(
                request.contains("x-token: secret") && request.contains("x-client: classic addon manager"),
                "{request}"
            );
        }
        let assets = assets.join().unwrap();
        assert!(!assets[0].contains("x-token") && !assets[0].contains("x-client"), "{}", assets[0]);
    }

    #[test]
    fn download_failure_after_redirect_maps_by_status() {
        let (listener, base) = listen();
        let server = serve(
            listener,
            vec![redirect("/gone"), "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()],
        );
        assert_eq!(download_from(&format!("{base}/a"), API_URL, "").err(), Some(ApiError::NotFound));
        server.join().unwrap();
    }

    #[test]
    fn envelope_null_or_missing_data_is_default() {
        assert_eq!(decode_envelope::<Vec<String>>(200, r#"{"status":true,"message":"ok","data":null}"#), Ok(vec![]));
        assert_eq!(decode_envelope::<Vec<String>>(200, r#"{"status":true,"message":"ok"}"#), Ok(vec![]));
        assert!(matches!(decode_envelope::<Vec<String>>(200, "not json"), Err(ApiError::Decode(_))));
        assert!(matches!(decode_envelope::<Vec<String>>(200, r#"{"status":true,"data":5}"#), Err(ApiError::Decode(_))));
    }

    #[test]
    fn large_envelope_decoding_agrees_with_the_tree() {
        #[derive(Debug, Default, PartialEq, Deserialize)]
        #[serde(default)]
        struct N {
            n: i64,
        }
        fn agree<T: DeserializeOwned + Default + PartialEq + fmt::Debug>(body: &str) {
            assert_eq!(decode_large_envelope::<T>(200, body), decode_envelope::<T>(200, body), "{body}");
        }

        // The common replies take the direct path.
        assert_eq!(decode_envelope_direct::<N>(200, r#"{"status":true,"data":{"n":3,"x":[1]}}"#), Some(Ok(N { n: 3 })));
        assert_eq!(
            decode_envelope_direct::<N>(200, r#"{"status":false,"message":"no","data":{"x":[1]}}"#),
            Some(Err(ApiError::Rejected { status: 200, message: "no".into() }))
        );
        // Inputs the two parsers read differently.
        for body in [
            r#"{"status":true,"data":{"n":1,"n":2}}"#,
            r#"{"status":true,"data":{"n":4,"x":1e400}}"#,
            r#"{"status":false,"message":"no","data":{"x":1e400}}"#,
            r#"{"status":true,"data":{"$serde_json::private::RawValue":"{\"n\":7}"}}"#,
            r#"{"status":true,"data":{"\u0024serde_json::private::RawValue":"{\"n\":7}"}}"#,
            r#"{"status":true,"data":{"n":"x"}}"#,
        ] {
            agree::<N>(body);
        }
        // `data` sits one level deep, so it may nest one level less than a whole body.
        for depth in [126, 127] {
            agree::<Value>(&format!(r#"{{"status":true,"data":{}{}}}"#, "[".repeat(depth), "]".repeat(depth)));
        }
    }
}
