//! The AI chat stream (`GET /ai/chat/stream`): a server-sent-events reader that turns the response into [`ChatEvent`]s
//! for the chat dialog.
use std::io;

use async_channel::Sender;
use serde_json::Value;
/// One dispatched server-sent event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

/// Incremental parser of the `text/event-stream` format (WHATWG HTML "interpreting an event stream").
#[derive(Default)]
pub struct SseParser {
    data: String,
    has_data: bool,
    event: String,
    first_line: bool,
}

impl SseParser {
    pub fn new() -> Self {
        Self { first_line: true, ..Default::default() }
    }

    /// Feeds one line (without its line terminator); returns the event when an empty line dispatches one.
    pub fn push_line(&mut self, line: &str) -> Option<SseEvent> {
        let mut line = line;
        if self.first_line {
            self.first_line = false;
            line = line.strip_prefix('\u{feff}').unwrap_or(line);
        }
        if line.is_empty() {
            let event = std::mem::take(&mut self.event);
            let has_data = std::mem::take(&mut self.has_data);
            let mut data = std::mem::take(&mut self.data);
            if !has_data {
                return None;
            }
            if data.ends_with('\n') {
                data.pop();
            }
            return Some(SseEvent { event: if event.is_empty() { "message".into() } else { event }, data });
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "data" => {
                self.data.push_str(value);
                self.data.push('\n');
                self.has_data = true;
            }
            "event" => self.event = value.to_string(),
            _ => {} // id / retry / unknown
        }
        None
    }
}

/// Conversation id and remaining message limit, carried by every JSON event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub conversation_id: Option<u64>,
    pub remaining_limit: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatEvent {
    Message {
        data: String,
        meta: Meta,
    },
    ToolCall {
        action: String,
        meta: Meta,
    },
    Complete {
        meta: Meta,
    },
    Error {
        message: Option<String>,
    },
    /// Data that was not JSON, treated as a raw chunk of reply text.
    Raw(String),
    /// Valid JSON of an unknown `type` (still counts as activity).
    Ignored,
}

fn parse_meta(v: &Value) -> Meta {
    Meta {
        conversation_id: v.get("conversation_id").and_then(Value::as_u64),
        remaining_limit: v
            .get("remaining_limit")
            .map(|n| n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(0)),
    }
}

/// Interprets the payload of one event: JSON is dispatched on its `type`, anything unparsable is a raw chunk.
pub fn parse_event_data(data: &str) -> ChatEvent {
    let Ok(v) = serde_json::from_str::<Value>(data) else {
        return ChatEvent::Raw(data.to_string());
    };
    if v.is_null() {
        return ChatEvent::Raw(data.to_string()); // JSON `null` has no `type`, so it is also a raw chunk
    }
    let text = |key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);
    match v.get("type").and_then(Value::as_str) {
        Some("message") => ChatEvent::Message { data: text("data").unwrap_or_default(), meta: parse_meta(&v) },
        Some("tool_call") => {
            ChatEvent::ToolCall { action: text("data").unwrap_or_else(|| "unknown_tool".into()), meta: parse_meta(&v) }
        }
        Some("complete") => ChatEvent::Complete { meta: parse_meta(&v) },
        Some("error") => ChatEvent::Error { message: text("message").filter(|m| !m.is_empty()) },
        _ => ChatEvent::Ignored,
    }
}

/// What the background reader sends to the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamMsg {
    Event(ChatEvent),
    /// Connection failed, non-200 status, read error, or the stream ended without `complete`.
    Failed,
}

/// Reads lines until the stream ends, forwarding events. Stops after `complete`/`error` (the client closes the
/// connection) or when the receiver is gone.
pub fn pump_lines(lines: impl Iterator<Item = io::Result<String>>, tx: &Sender<StreamMsg>) {
    let mut parser = SseParser::new();
    for line in lines {
        let Ok(line) = line else {
            let _ = tx.send_blocking(StreamMsg::Failed);
            return;
        };
        let Some(ev) = parser.push_line(&line) else { continue };
        if ev.event != "message" {
            continue;
        }
        let event = parse_event_data(&ev.data);
        let terminal = matches!(event, ChatEvent::Complete { .. } | ChatEvent::Error { .. });
        if tx.send_blocking(StreamMsg::Event(event)).is_err() || terminal {
            return;
        }
    }
    let _ = tx.send_blocking(StreamMsg::Failed);
}

/// Blocking: opens `/ai/chat/stream?...` and pumps it into `tx` (run on a background thread).
pub fn run_stream(path: &str, tx: &Sender<StreamMsg>) {
    match super::open_stream(path) {
        Ok(stream) => pump_lines(stream, tx),
        Err(e) => {
            log::error!("Opening the chat stream failed: {e:#}");
            let _ = tx.send_blocking(StreamMsg::Failed);
        }
    }
}

/// Request path for the chat stream: `/ai/chat/stream?p=..&client=..&conversation_id=..`, form-urlencoded. The token
/// goes in the `X-Token` header that every API request carries.
pub fn stream_path(message: &str, version: &str, conversation_id: Option<u64>) -> String {
    let mut q = form_urlencoded::Serializer::new(String::new());
    q.append_pair("p", message).append_pair("client", version);
    if let Some(id) = conversation_id {
        q.append_pair("conversation_id", &id.to_string());
    }
    format!("/ai/chat/stream?{}", q.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
    };

    fn lines(s: &str) -> impl Iterator<Item = io::Result<String>> + '_ {
        s.lines().map(|l| Ok(l.to_string()))
    }

    fn collect(stream: &str) -> Vec<StreamMsg> {
        let (tx, rx) = async_channel::unbounded();
        pump_lines(lines(stream), &tx);
        drop(tx);
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    #[test]
    fn sse_parser_basics() {
        let mut p = SseParser::new();
        assert_eq!(p.push_line(": comment"), None);
        assert_eq!(p.push_line("data: hello"), None);
        assert_eq!(p.push_line("data:world"), None);
        let ev = p.push_line("").unwrap();
        assert_eq!(ev, SseEvent { event: "message".into(), data: "hello\nworld".into() });
        // blank line without data dispatches nothing
        assert_eq!(p.push_line(""), None);
        p.push_line("event: ping");
        p.push_line("data: x");
        assert_eq!(p.push_line("").unwrap().event, "ping");
        // field without colon, leading space stripped once only
        p.push_line("data");
        p.push_line("data:  two");
        assert_eq!(p.push_line("").unwrap().data, "\n two");
    }

    #[test]
    fn event_parsing() {
        assert_eq!(
            parse_event_data(r#"{"type":"message","data":"Hi","conversation_id":8,"remaining_limit":7}"#),
            ChatEvent::Message { data: "Hi".into(), meta: Meta { conversation_id: Some(8), remaining_limit: Some(7) } }
        );
        assert_eq!(
            parse_event_data(r#"{"type":"tool_call","data":"list_addons"}"#),
            ChatEvent::ToolCall { action: "list_addons".into(), meta: Meta::default() }
        );
        assert_eq!(
            parse_event_data(r#"{"type":"tool_call","data":5}"#),
            ChatEvent::ToolCall { action: "unknown_tool".into(), meta: Meta::default() }
        );
        assert_eq!(parse_event_data(r#"{"type":"error"}"#), ChatEvent::Error { message: None });
        assert_eq!(parse_event_data(r#"{"type":"zzz"}"#), ChatEvent::Ignored);
        assert_eq!(parse_event_data("plain text"), ChatEvent::Raw("plain text".into()));
        assert_eq!(parse_event_data("null"), ChatEvent::Raw("null".into()));
    }

    #[test]
    fn pump_stops_after_complete() {
        let msgs = collect(
            "data: {\"type\":\"message\",\"data\":\"a\"}\n\ndata: {\"type\":\"complete\",\"remaining_limit\":3}\n\ndata: {\"type\":\"message\",\"data\":\"late\"}\n\n",
        );
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[1], StreamMsg::Event(ChatEvent::Complete { .. })));
    }

    #[test]
    fn pump_reports_failure_when_stream_ends_without_complete() {
        let msgs = collect("data: {\"type\":\"message\",\"data\":\"a\"}\n\n");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1], StreamMsg::Failed);
    }

    #[test]
    fn stream_path_encoding() {
        assert_eq!(
            stream_path("hello world & more?", "3.2.1", Some(8)),
            "/ai/chat/stream?p=hello+world+%26+more%3F&client=3.2.1&conversation_id=8"
        );
        assert_eq!(stream_path("x", "1", None), "/ai/chat/stream?p=x&client=1");
    }

    /// Real HTTP round trip against a local mock server (no live API).
    #[test]
    fn mock_http_server_sse() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(sock.try_clone().unwrap());
            let mut line = String::new();
            let mut request_line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                if request_line.is_empty() {
                    request_line = line.clone();
                }
                line.clear();
            }
            assert!(request_line.starts_with("GET /ai/chat/stream?p=hi"), "{request_line}");
            let body = "data: {\"type\":\"tool_call\",\"data\":\"list_addons\"}\n\n\
                        data: {\"type\":\"message\",\"data\":\"Hel\",\"conversation_id\":8}\r\n\r\n\
                        : keep-alive\n\n\
                        data: {\"type\":\"message\",\"data\":\"lo\"}\n\n\
                        data: {\"type\":\"complete\",\"remaining_limit\":9}\n\n";
            write!(sock, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}")
                .unwrap();
        });
        let agent = ureq::Agent::new_with_defaults();
        let resp = agent.get(format!("http://127.0.0.1:{port}/ai/chat/stream?p=hi")).call().unwrap();
        let reader: Box<dyn io::Read + Send> = Box::new(resp.into_body().into_reader());
        let it = BufReader::new(reader).lines();
        let (tx, rx) = async_channel::unbounded();
        pump_lines(it, &tx);
        server.join().unwrap();
        let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        let event = |e| StreamMsg::Event(e);
        assert_eq!(
            msgs,
            [
                event(ChatEvent::ToolCall { action: "list_addons".into(), meta: Meta::default() }),
                event(ChatEvent::Message {
                    data: "Hel".into(),
                    meta: Meta { conversation_id: Some(8), remaining_limit: None }
                }),
                event(ChatEvent::Message { data: "lo".into(), meta: Meta::default() }),
                event(ChatEvent::Complete { meta: Meta { conversation_id: None, remaining_limit: Some(9) } }),
            ]
        );
    }
}
