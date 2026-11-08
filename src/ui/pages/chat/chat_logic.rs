//! Pure (UI independent) chat logic: turn and tool-call types, tool descriptions, the conversation state machine,
//! the gradual text reveal, and the server-sent-events reader for the chat stream.
use std::{collections::HashSet, io};

use async_channel::Sender;
use serde_json::Value;

use crate::ui::icons::{self, Lucide};

// ------------------------------------------------------------------------------------------------ types

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Done,
}

/// The stream only tells us that a tool was invoked, never that it finished, so `status` is inferred from whatever
/// event arrives next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub action: String,
    pub status: ToolStatus,
}

/// One exchange: the user's message, the tools called while answering it, and the reply received so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    pub user: String,
    pub tools: Vec<ToolCall>,
    pub assistant: String,
}

/// A part of the conversation that plays its enter animation, addressed by turn index and tool call index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    User(usize),
    Reply(usize),
    Tool(usize, usize),
}

// ------------------------------------------------------------------------------------------------ tool catalog

pub struct ToolMeta {
    /// Shown while the call is still in flight.
    pub running: &'static str,
    /// Shown once the turn has moved past this call.
    pub done: &'static str,
    pub icon: Lucide,
}

pub fn tool_meta(action: &str) -> ToolMeta {
    match action {
        "list_addons" => ToolMeta { running: "Getting addons...", done: "Listed addons", icon: icons::PACKAGE_SEARCH },
        "get_latest_release" => {
            ToolMeta { running: "Checking the latest release...", done: "Checked the latest release", icon: icons::TAG }
        }
        "get_addon_readme" => {
            ToolMeta { running: "Opening the addon README...", done: "Read the addon README", icon: icons::FILE_TEXT }
        }
        "get_addon_manager_documentation" => ToolMeta {
            running: "Fetching manager instructions...",
            done: "Read the manager instructions",
            icon: icons::BOOK_TEXT,
        },
        "get_addon_details" => {
            ToolMeta { running: "Gathering addon details...", done: "Gathered addon details", icon: icons::INFO }
        }
        "search_archeage_wiki" => ToolMeta {
            running: "Searching the ArcheAge Classic wiki...",
            done: "Searched the ArcheAge Classic wiki",
            icon: icons::SEARCH,
        },
        "get_archeage_wiki_page_content" => {
            ToolMeta { running: "Reading the wiki page...", done: "Read a wiki page", icon: icons::SCROLL_TEXT }
        }
        _ => ToolMeta { running: "Working on something...", done: "Used a tool", icon: icons::WRENCH },
    }
}

// ------------------------------------------------------------------------------------------------ SSE

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
    match crate::backend::api::open_stream(path) {
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

// ------------------------------------------------------------------------------------------------ chat state

pub const SLOW_RESPONSE_TEXT: &str = "Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸";
pub const GENERIC_ERROR_TEXT: &str = "Sorry, I encountered an error processing your request.";

/// The conversation state: turns, whether a reply is pending, conversation id and remaining limit. The connection
/// itself is handled by `AiChatDialog`, and stream events always belong to the last turn.
#[derive(Default)]
pub struct ChatState {
    pub turns: Vec<Turn>,
    pub waiting: bool,
    pub conversation_id: Option<u64>,
    pub remaining_limit: i64,
    /// Parts that should play their enter animation.
    pub animating: HashSet<Part>,
    /// The last reply holds the slow-response text, which the first received text replaces.
    slow_text: bool,
}

impl ChatState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the tool calls of the last turn as done. Earlier turns were settled before the next one began.
    fn mark_tools_done(&mut self) {
        if let Some(turn) = self.turns.last_mut() {
            for call in &mut turn.tools {
                call.status = ToolStatus::Done;
            }
        }
    }

    /// Appends received text to the last reply, replacing the slow-response text.
    fn push_reply(&mut self, text: &str) {
        let slow_text = std::mem::take(&mut self.slow_text);
        if let Some(turn) = self.turns.last_mut() {
            if slow_text {
                turn.assistant.clear();
            }
            turn.assistant.push_str(text);
        }
    }

    fn set_reply(&mut self, text: String) {
        self.slow_text = false;
        if let Some(turn) = self.turns.last_mut() {
            turn.assistant = text;
        }
    }

    fn sync_meta(&mut self, meta: &Meta) {
        if let Some(id) = meta.conversation_id {
            self.conversation_id = Some(id);
        }
        if let Some(limit) = meta.remaining_limit {
            self.remaining_limit = limit;
        }
    }

    /// Settles tool calls and stops waiting, after the connection was closed.
    pub fn cleanup(&mut self) {
        self.mark_tools_done();
        self.waiting = false;
    }

    /// Starts a new conversation: settles tool calls and clears the turns and the conversation id.
    pub fn reset(&mut self) {
        self.cleanup();
        self.turns.clear();
        self.slow_text = false;
        self.conversation_id = None;
        self.animating.clear();
    }

    /// Starts a turn with the user's message and an empty reply.
    pub fn begin_send(&mut self, message: &str) {
        self.cleanup();
        let i = self.turns.len();
        self.turns.push(Turn { user: message.to_string(), ..Turn::default() });
        self.animating.extend([Part::User(i), Part::Reply(i)]);
        self.slow_text = false;
        self.waiting = true;
    }

    /// Applies one stream event to the last turn. Returns `true` when the stream is finished (the connection must be
    /// closed).
    pub fn apply(&mut self, event: ChatEvent) -> bool {
        match event {
            ChatEvent::Message { data, meta } => {
                self.push_reply(&data);
                self.mark_tools_done();
                self.sync_meta(&meta);
                false
            }
            ChatEvent::ToolCall { action, meta } => {
                self.mark_tools_done();
                let n = self.turns.len();
                if let Some(turn) = self.turns.last_mut() {
                    self.animating.insert(Part::Tool(n - 1, turn.tools.len()));
                    turn.tools.push(ToolCall { action, status: ToolStatus::Running });
                }
                self.sync_meta(&meta);
                false
            }
            ChatEvent::Complete { meta } => {
                self.sync_meta(&meta);
                self.cleanup();
                true
            }
            ChatEvent::Error { message } => {
                self.set_reply(format!(
                    "Sorry, I encountered an error: {}",
                    message.as_deref().unwrap_or("Server error")
                ));
                self.cleanup();
                true
            }
            ChatEvent::Raw(raw) => {
                self.push_reply(&raw);
                self.mark_tools_done();
                false
            }
            ChatEvent::Ignored => false,
        }
    }

    /// The stream failed or could not be opened: keeps the text received so far, or shows a generic error if none.
    pub fn stream_failed(&mut self) {
        if self.slow_text || self.turns.last().is_some_and(|t| t.assistant.is_empty()) {
            self.set_reply(GENERIC_ERROR_TEXT.to_string());
        }
        self.cleanup();
    }

    /// The 10 s "slow response" fallback text (only while the reply is still empty).
    pub fn slow_response(&mut self) {
        if let Some(turn) = self.turns.last_mut()
            && turn.assistant.is_empty()
        {
            turn.assistant = SLOW_RESPONSE_TEXT.to_string();
            self.slow_text = true;
        }
    }
}

// ------------------------------------------------------------------------------------------------ text reveal

pub const REVEAL_TICK_MS: u64 = 16;
/// How far behind the real content the reveal is allowed to trail.
const CATCHUP_WINDOW_MS: f64 = 1000.0;

fn chars_per_tick(backlog: usize) -> usize {
    let ticks_available = CATCHUP_WINDOW_MS / REVEAL_TICK_MS as f64;
    ((backlog as f64 / ticks_available).ceil() as usize).max(1)
}

/// Reveals streamed text gradually, at a rate that catches up with the real content within about 1 s.
pub struct TextReveal {
    text: String,
    /// Chars revealed, and the byte offset in `text` where they end.
    cursor: usize,
    end: usize,
    /// Chars in `text`.
    len: usize,
    has_streamed: bool,
    step: usize,
}

impl TextReveal {
    pub fn new(text: &str, is_streaming: bool) -> Self {
        let len = text.chars().count();
        let (cursor, end) = if is_streaming { (0, 0) } else { (len, text.len()) };
        Self { text: text.to_string(), cursor, end, len, has_streamed: is_streaming, step: chars_per_tick(len) }
    }

    /// Adopts the latest text. Called on every render and does nothing when nothing changed.
    pub fn sync(&mut self, text: &str, is_streaming: bool) {
        if self.text == text && (!is_streaming || self.has_streamed) {
            return;
        }
        let has_streamed = self.has_streamed || is_streaming;
        let len = text.chars().count();
        let (cursor, end) = if !has_streamed {
            (len, text.len())
        } else if text.starts_with(&self.text) {
            (self.cursor, self.end)
        } else {
            (0, 0)
        };
        self.text = text.to_string();
        self.len = len;
        self.cursor = cursor;
        self.end = end;
        self.has_streamed = has_streamed;
        self.step = chars_per_tick(len.saturating_sub(cursor));
    }

    pub fn complete(&mut self) {
        self.cursor = self.len;
        self.end = self.text.len();
    }

    pub fn is_catching_up(&self) -> bool {
        self.has_streamed && self.cursor < self.len
    }

    /// One 16 ms interval tick. Only the newly revealed chars are walked.
    pub fn tick(&mut self) {
        if self.cursor < self.len {
            let step = self.step.min(self.len - self.cursor);
            let rest = &self.text[self.end..];
            self.end += rest.char_indices().nth(step).map_or(rest.len(), |(i, _)| i);
            self.cursor += step;
        }
    }

    pub fn displayed(&self) -> &str {
        &self.text[..self.end]
    }

    pub fn is_revealing(&self, is_streaming: bool) -> bool {
        self.has_streamed && (is_streaming || self.is_catching_up())
    }
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

    fn reply(s: &ChatState) -> &str {
        &s.turns.last().unwrap().assistant
    }

    fn tool(action: &str) -> ChatEvent {
        ChatEvent::ToolCall { action: action.into(), meta: Meta::default() }
    }

    fn chunk(data: &str) -> ChatEvent {
        ChatEvent::Message { data: data.into(), meta: Meta::default() }
    }

    #[test]
    fn state_flow_with_tools() {
        let mut s = ChatState::new();
        s.begin_send("hi");
        assert!(s.waiting);
        assert!(!s.apply(tool("list_addons")));
        assert!(!s.apply(tool("get_addon_details")));
        // the first tool is settled when the next one arrives
        let statuses = |s: &ChatState| s.turns[0].tools.iter().map(|t| t.status).collect::<Vec<_>>();
        assert_eq!(statuses(&s), [ToolStatus::Done, ToolStatus::Running]);
        assert!(s.animating.contains(&Part::Tool(0, 1)));
        let meta = Meta { conversation_id: Some(8), remaining_limit: Some(4) };
        s.apply(ChatEvent::Message { data: "Hel".into(), meta });
        s.apply(chunk("lo"));
        assert_eq!(reply(&s), "Hello");
        assert_eq!(statuses(&s), [ToolStatus::Done, ToolStatus::Done]);
        assert_eq!(s.conversation_id, Some(8));
        assert_eq!(s.remaining_limit, 4);
        assert!(s.apply(ChatEvent::Complete { meta: Meta::default() }));
        assert!(!s.waiting);

        // the next turn receives the following events, the finished one keeps its reply and tools
        s.begin_send("more");
        s.apply(tool("search_addons"));
        s.apply(chunk("Bye"));
        assert_eq!(s.turns.len(), 2);
        assert_eq!((s.turns[0].assistant.as_str(), s.turns[0].tools.len()), ("Hello", 2));
        let last = &s.turns[1];
        assert_eq!((last.user.as_str(), last.assistant.as_str(), last.tools.len()), ("more", "Bye", 1));
        assert!(s.animating.contains(&Part::Tool(1, 0)));
    }

    #[test]
    fn state_errors() {
        let mut s = ChatState::new();
        s.begin_send("hi");
        s.apply(chunk("partial"));
        assert!(s.apply(ChatEvent::Error { message: None }));
        assert_eq!(reply(&s), "Sorry, I encountered an error: Server error");

        let mut s = ChatState::new();
        s.begin_send("hi");
        s.stream_failed();
        assert_eq!(reply(&s), GENERIC_ERROR_TEXT);
        assert!(!s.waiting);

        let mut s = ChatState::new();
        s.begin_send("hi");
        s.apply(chunk("partial"));
        s.stream_failed();
        assert_eq!(reply(&s), "partial");

        let mut s = ChatState::new();
        s.begin_send("hi");
        s.slow_response();
        assert_eq!(reply(&s), SLOW_RESPONSE_TEXT);
        // a later chunk replaces the fallback text
        s.apply(chunk("ok"));
        s.apply(chunk("!"));
        assert_eq!(reply(&s), "ok!");

        // a failure with only the fallback text shown counts as nothing received
        let mut s = ChatState::new();
        s.begin_send("hi");
        s.slow_response();
        s.stream_failed();
        assert_eq!(reply(&s), GENERIC_ERROR_TEXT);
    }

    #[test]
    fn reveal_catches_up() {
        let mut r = TextReveal::new("", true);
        r.sync("hello world", true);
        assert_eq!(r.displayed(), "");
        assert!(r.is_catching_up());
        assert!(r.is_revealing(true));
        r.tick();
        assert_eq!(r.displayed(), "h");
        r.sync("hello world, more", true);
        assert_eq!(r.displayed(), "h"); // prefix kept
        r.complete();
        assert_eq!(r.displayed(), "hello world, more");
        assert!(!r.is_catching_up());
        assert!(r.is_revealing(true));
        assert!(!r.is_revealing(false));
        // content replaced -> restart
        r.sync("other", true);
        assert_eq!(r.displayed(), "");
        // never streamed: shows everything
        let mut r = TextReveal::new("abc", false);
        assert_eq!(r.displayed(), "abc");
        r.sync("abcdef", false);
        assert_eq!(r.displayed(), "abcdef");
        assert!(!r.is_revealing(false));
        // multi-byte
        let mut r = TextReveal::new("", true);
        r.sync("héé🐸", true);
        r.tick();
        r.tick();
        assert_eq!(r.displayed(), "hé");
        // the revealed part survives growing text
        r.sync("héé🐸 ok", true);
        r.tick();
        r.tick();
        assert_eq!(r.displayed(), "héé🐸");
    }

    #[test]
    fn large_backlog_step() {
        assert_eq!(chars_per_tick(0), 1);
        assert_eq!(chars_per_tick(62), 1);
        assert_eq!(chars_per_tick(63), 2);
        assert_eq!(chars_per_tick(625), 10);
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
        let mut state = ChatState::new();
        state.begin_send("hi");
        let mut finished = false;
        while let Ok(StreamMsg::Event(ev)) = rx.try_recv() {
            finished = state.apply(ev);
        }
        assert!(finished);
        assert_eq!(reply(&state), "Hello");
        assert_eq!(state.conversation_id, Some(8));
        assert_eq!(state.remaining_limit, 9);
    }
}
