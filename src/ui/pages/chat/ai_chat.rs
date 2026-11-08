//! The "Daru Informational Network" chat dialog.
//!
//! The dialog body is this view (`AiChatDialog`). `UserBar` creates it once per signed-in session, so the conversation
//! survives closing the dialog, and shows it with `dialog::open(.., content_view(chat))`.
//!
//! Streaming: `chat_logic::run_stream` reads the server-sent-events response on a dedicated thread and feeds an
//! `async_channel`. A UI-side task applies the events to [`ChatState`]. Dropping that task (close, stop, new
//! conversation) closes the connection at the next event.
use std::time::Duration;

use gpui_kit::component::{Icon, Sizable as _, avatar::Avatar, clipboard::Clipboard};
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, AppContext as _, Context, Entity, Focusable, FontWeight,
    InteractiveElement, IntoElement, ObjectFit, ParentElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, StyledImage as _, Task, Transformation, Window, div, ease_out_quint, img,
    percentage, prelude::FluentBuilder as _, px,
};

use super::chat_logic::{
    ChatState, Part, REVEAL_TICK_MS, StreamMsg, TextReveal, ToolCall, ToolStatus, Turn, run_stream, stream_path,
    tool_meta,
};
use crate::{
    backend::api::User,
    state::{self, toast},
    ui::{
        assets::images,
        components::{
            Button, ButtonSize, ButtonVariant, EmptyState, Input, InputState, Markdown, MarkdownStyle, ScrollArea,
            dialog, floating::window_area, on_input_enter, on_input_focus, spinner, text_input,
        },
        icons,
        theme::{TokensExt, TwText, alpha, radius},
    },
};

const PLACEHOLDER: &str = "Ask your question, the Darus won't judge...";

pub struct AiChatDialog {
    st: ChatState,
    reveal: TextReveal,
    reveal_running: bool,
    reveal_task: Option<Task<()>>,
    stream_task: Option<Task<()>>,
    slow_task: Option<Task<()>>,
    anim_task: Option<Task<()>>,
    scroll_task: Option<Task<()>>,
    /// Indices of turns whose tool activity list is expanded.
    expanded_tools: std::collections::HashSet<usize>,
    input: Entity<InputState>,
    placeholder_waiting: bool,
    scroll: ScrollHandle,
    /// Scroll to the bottom on the next render.
    force_scroll: bool,
    /// Keep scrolling to new content. Turns off when the user scrolls up and back on at the bottom.
    stick_to_bottom: bool,
    /// Scroll position at the last check, used to notice when the user scrolls up.
    last_scroll_y: Option<gpui_kit::Pixels>,
    /// Widest a message bubble may get: 80% of the message column, in pixels, because gpui measures wrapped text
    /// wrongly with percentage widths.
    bubble_max: gpui_kit::Pixels,
}

impl AiChatDialog {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = text_input(window, cx, PLACEHOLDER);
        on_input_enter(&input, window, cx, |this: &mut Self, _, window, cx| this.submit(window, cx));
        // Repaint when the input gains or loses focus, so its focus styling updates.
        on_input_focus(&input, window, cx, |_, _, _, cx| cx.notify());
        Self {
            st: ChatState::new(),
            reveal: TextReveal::new("", false),
            reveal_running: false,
            reveal_task: None,
            stream_task: None,
            slow_task: None,
            anim_task: None,
            scroll_task: None,
            expanded_tools: Default::default(),
            input,
            placeholder_waiting: false,
            scroll: ScrollHandle::new(),
            force_scroll: true,
            stick_to_bottom: true,
            last_scroll_y: None,
            bubble_max: px(512.0),
        }
    }

    /// Focuses the input unless it is disabled because a reply is pending.
    pub fn focus_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.st.waiting {
            self.input.update(cx, |s, cx| s.focus(window, cx));
        }
    }

    /// Called right after the dialog was opened.
    pub fn on_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pin_to_bottom();
        self.focus_input(window, cx);
        // the dialog layer grabs focus while it opens; take it back on the next frame
        let this = cx.entity();
        window.on_next_frame(move |window, cx| {
            window.on_next_frame(move |window, cx| this.update(cx, |t, cx| t.focus_input(window, cx)));
            let _ = cx;
        });
        cx.notify();
    }

    /// Called when the dialog closes: ends any running stream and collapses all tool activity lists.
    pub fn on_closed(&mut self, cx: &mut Context<Self>) {
        self.cleanup_connection(cx);
        self.expanded_tools.clear();
    }

    /// Closes the dialog from its close button.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.on_closed(cx);
        dialog::close(window, cx);
    }

    // ------------------------------------------------------------------------------------------ connection

    /// Stops the stream and the slow-response timer, settles tool calls and stops waiting.
    pub fn cleanup_connection(&mut self, cx: &mut Context<Self>) {
        self.stream_task = None;
        self.slow_task = None;
        self.st.cleanup();
        cx.notify();
    }

    /// Starts a new conversation: stops any stream and clears the history and the conversation id.
    fn reset_conversation(&mut self, cx: &mut Context<Self>) {
        self.stream_task = None;
        self.slow_task = None;
        self.st.reset();
        self.pin_to_bottom();
        self.expanded_tools.clear();
        cx.notify();
    }

    /// Stop button: ends the stream and shows the text received so far at once.
    fn stop(&mut self, cx: &mut Context<Self>) {
        self.cleanup_connection(cx);
        self.reveal.complete();
    }

    /// Sends the typed text and clears the input, unless a reply is pending or the text is blank.
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.st.waiting {
            return;
        }
        let text = self.input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.reveal.complete();
        self.send_message(&text, cx);
        self.input.update(cx, |s, cx| s.set_value("", window, cx));
    }

    /// Starts a turn with the user message and an empty reply, then opens the stream and starts the slow-response
    /// timer.
    fn send_message(&mut self, message: &str, cx: &mut Context<Self>) {
        if message.trim().is_empty() || self.st.waiting {
            return;
        }
        self.cleanup_connection(cx);
        self.st.begin_send(message);
        self.after_history_change(true, cx);

        // Open the stream on a background thread and apply its events on the UI side. Dropping the tasks before the
        // next turn begins keeps their events and timer on this turn.
        let version = state::app_info(cx).read(cx).version.clone();
        let path = stream_path(message, &version, self.st.conversation_id);
        let (tx, rx) = async_channel::unbounded::<StreamMsg>();
        std::thread::spawn(move || run_stream(&path, &tx));

        self.stream_task = Some(cx.spawn(async move |this, cx| {
            while let Ok(msg) = rx.recv().await {
                let finished = this.update(cx, |this, cx| this.on_stream_msg(msg, cx));
                if finished.unwrap_or(true) {
                    break;
                }
            }
        }));
        // If nothing arrives within 10 seconds, show the slow-response text.
        self.slow_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(10)).await;
            let _ = this.update(cx, |this, cx| {
                this.st.slow_response();
                this.after_history_change(false, cx);
            });
        }));
    }

    /// Returns `true` when the stream is finished.
    fn on_stream_msg(&mut self, msg: StreamMsg, cx: &mut Context<Self>) -> bool {
        self.slow_task = None;
        let finished = match msg {
            StreamMsg::Event(ev) => self.st.apply(ev),
            StreamMsg::Failed => {
                self.st.stream_failed();
                true
            }
        };
        self.after_history_change(false, cx);
        finished
    }

    /// Schedules follow-up work after the history changed: scrolling after 50 ms and, while messages are animating in,
    /// clearing the animation set after 1 s.
    ///
    /// `jump` is set when the user sends a message and always scrolls to the bottom.
    /// Other updates only scroll if the user is still at the bottom.
    fn after_history_change(&mut self, jump: bool, cx: &mut Context<Self>) {
        if jump {
            self.pin_to_bottom();
        }
        self.scroll_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(50)).await;
            let _ = this.update(cx, |this, cx| {
                if jump {
                    this.force_scroll = true;
                } else {
                    this.follow_new_content();
                }
                cx.notify();
            });
        }));
        if !self.st.animating.is_empty() {
            self.anim_task = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(1000)).await;
                let _ = this.update(cx, |this, cx| {
                    this.st.animating.clear();
                    cx.notify();
                });
            }));
        }
        cx.notify();
    }

    fn ensure_reveal_ticker(&mut self, cx: &mut Context<Self>) {
        if self.reveal_running || !self.reveal.is_catching_up() {
            return;
        }
        self.reveal_running = true;
        self.reveal_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(REVEAL_TICK_MS)).await;
                let keep = this
                    .update(cx, |this, cx| {
                        // new text makes the content taller
                        this.follow_new_content();
                        this.reveal.tick();
                        let keep = this.reveal.is_catching_up();
                        this.reveal_running = keep;
                        cx.notify();
                        keep
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        }));
    }

    /// Scroll to the bottom and keep following new content.
    fn pin_to_bottom(&mut self) {
        self.force_scroll = true;
        self.stick_to_bottom = true;
        self.last_scroll_y = None;
    }

    /// Scrolls to new content unless the user has scrolled up.
    /// The app itself only ever scrolls down, so any upward move since the last check came from the user.
    fn follow_new_content(&mut self) {
        let y = self.scroll.offset().y;
        if self.last_scroll_y.is_some_and(|last| y > last + px(0.5)) {
            self.stick_to_bottom = false;
        }
        if self.scroll.max_offset().y + y < px(1.0) {
            self.stick_to_bottom = true;
        }
        self.last_scroll_y = Some(y);
        if self.stick_to_bottom {
            self.force_scroll = true;
        }
    }
}

/// The user's Discord avatar at `size`. Without a custom Discord avatar it shows the username's initials.
/// The kit avatar has no loading or error fallback: while the image loads, or when it fails, the circle stays empty.
pub(crate) fn user_avatar(user: &User, size: Pixels) -> Avatar {
    let avatar = Avatar::new().with_size(size).name(user.username.clone());
    if user.avatar.is_empty() {
        avatar
    } else {
        avatar.src(format!("https://cdn.discordapp.com/avatars/{}/{}.png", user.discord_id, user.avatar))
    }
}

/// Fade-in with a slide up: opacity goes from 0 to 1 while the element rises from `from` px below its final spot.
fn enter_animation(el: gpui_kit::Div, part: Part, duration_ms: u64, from: f32) -> AnyElement {
    let id = match part {
        Part::User(i) => format!("enter-user-{i}"),
        Part::Reply(i) => format!("enter-reply-{i}"),
        Part::Tool(i, j) => format!("enter-tool-{i}-{j}"),
    };
    el.relative()
        .with_animation(
            SharedString::from(id),
            Animation::new(Duration::from_millis(duration_ms)).with_easing(ease_out_quint()),
            move |el, t| el.opacity(t).top(px(from * (1.0 - t))),
        )
        .into_any_element()
}

impl Render for AiChatDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let area = window_area(window);

        // ---- derived state: revealed text and which turn is active
        let waiting = self.st.waiting;
        self.reveal.sync(self.st.turns.last().map_or("", |turn| turn.assistant.as_str()), waiting);
        self.ensure_reveal_ticker(cx);
        let is_revealing = self.reveal.is_revealing(waiting);
        let active_turn = if waiting || is_revealing { self.st.turns.len().checked_sub(1) } else { None };
        let has_turns = !self.st.turns.is_empty();

        // ---- input placeholder follows the status text
        let status = if waiting {
            "Daru is thinking..."
        } else if is_revealing {
            "Daru is responding..."
        } else {
            "Darus are known for their wisdom, but sometimes even they make mistakes."
        };
        if self.placeholder_waiting != waiting {
            self.placeholder_waiting = waiting;
            let text = if waiting { status } else { PLACEHOLDER };
            self.input.update(cx, |s, cx| s.set_placeholder(text, window, cx));
            if !waiting {
                self.focus_input(window, cx);
            }
        }

        // ---- auto scroll, only when requested and not on every render
        if has_turns && self.force_scroll {
            self.scroll.scroll_to_bottom();
        }
        self.force_scroll = false;

        // ---- sizes
        // 650 px tall, or 80% of the window height from 1024 px wide up, never taller than the window minus 64 px.
        let height = if area.size.width >= px(1024.0) { area.size.height * 0.8 } else { px(650.0) };
        let height = height.min((area.size.height - px(64.0)).max(px(0.0)));
        let wide = area.size.width >= px(640.0); // roomier padding from 640 px wide
        {
            let frame_w = (area.size.width - px(32.0)).min(area.size.width * 0.7);
            self.bubble_max = ((frame_w - px(2.0) - px(if wide { 48.0 } else { 32.0 })) * 0.8).max(px(0.0));
        }

        // ---- header
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(if wide { 24.0 } else { 16.0 }))
            .py(px(16.0))
            .border_b_1()
            .border_color(alpha(t.border, 0.4))
            .child(
                div()
                    .child(
                        div()
                            .text_lg_tw()
                            .line_height(px(18.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Daru Informational Network"),
                    )
                    .child(
                        div()
                            .mt(px(8.0))
                            .text_sm_tw()
                            .text_color(t.muted_foreground)
                            .child("The Darus have information, if you have coin."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::new("chat-new")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Icon)
                            .w(px(28.0))
                            .h(px(28.0))
                            .text_color(t.muted_foreground)
                            .icon(icons::MESSAGE_SQUARE_PLUS)
                            .disabled(!has_turns)
                            .on_click(cx.listener(|this, _, _, cx| this.reset_conversation(cx))),
                    )
                    .child(
                        Button::new("chat-close")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Icon)
                            .w(px(28.0))
                            .h(px(28.0))
                            .icon(icons::X)
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            );

        // ---- messages
        let body: AnyElement = if self.st.turns.is_empty() {
            empty_state(window, cx).into_any_element()
        } else {
            let mut list = div().flex().flex_col().gap(px(24.0));
            for (i, turn) in self.st.turns.iter().enumerate() {
                list = list.child(self.user_message(i, &turn.user, cx));
                if let Some(el) = self.assistant_turn(i, turn, active_turn == Some(i), is_revealing, cx) {
                    list = list.child(el);
                }
            }
            list.into_any_element()
        };
        let pad = if wide { 24.0 } else { 16.0 };
        let scroll_area = ScrollArea::new("chat-scroll")
            .track_scroll(&self.scroll)
            .flex_1()
            .min_h_0()
            .content_style(move |d| d.p(px(pad)).flex().flex_col())
            .child(body);

        // ---- input
        let input = Input::new(&self.input)
            .disabled(waiting)
            .h(px(42.0))
            .px(px(16.0))
            .pr(px(64.0))
            .py(px(10.0))
            .rounded_full()
            .bg(alpha(t.muted, 0.3))
            .border_color(t.border)
            .shadow(vec![]);
        let action_button: AnyElement = if waiting {
            Button::new("chat-stop")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .rounded_full()
                .on_click(cx.listener(|this, _, _, cx| this.stop(cx)))
                // Stop icon: a filled rounded square.
                .child(div().size(px(11.7)).rounded(px(1.2)).bg(t.primary))
                .into_any_element()
        } else {
            Button::new("chat-send")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .rounded_full()
                .text_color(t.primary)
                .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx)))
                .child(Icon::new(icons::SEND).size(px(18.0)))
                .into_any_element()
        };
        let remaining = self.st.remaining_limit;
        let input_section = div()
            .flex()
            .flex_col()
            .when(remaining > 0, |d| {
                d.child(
                    div().flex().justify_end().px(px(16.0)).pb(px(8.0)).child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .rounded_full()
                            .bg(alpha(t.muted, 0.2))
                            .px(px(8.0))
                            .py(px(2.0))
                            .text_xs_tw()
                            .text_color(alpha(t.muted_foreground, 0.6))
                            .child(format!("{remaining} messages remaining")),
                    ),
                )
            })
            .child(
                div()
                    .border_t_1()
                    .border_color(alpha(t.border, 0.4))
                    .px(px(pad))
                    .pt(px(if wide { 16.0 } else { 12.0 }))
                    .pb(px(16.0))
                    .child(
                        div().flex().items_center().gap(px(8.0)).child(
                            div()
                                .relative()
                                .flex_1()
                                .child(input)
                                .child(div().absolute().top(px(3.0)).right(px(6.0)).child(action_button)),
                        ),
                    )
                    .child(div().mt(px(8.0)).flex().min_h(px(24.0)).items_center().justify_center().child(
                        div().text_center().text_xs_tw().text_color(alpha(t.muted_foreground, 0.8)).child(status),
                    )),
            );

        div().flex().flex_col().w_full().h(height).child(header).child(scroll_area).child(input_section)
    }
}

impl AiChatDialog {
    /// The user message of turn `i`: right-aligned bubble next to the user's avatar.
    fn user_message(&self, i: usize, message: &str, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let user = state::user(cx).read(cx).user.clone();
        let row = div()
            .flex()
            .items_start()
            .justify_end()
            .gap(px(12.0))
            .child(
                div()
                    .max_w(self.bubble_max)
                    .rounded(px(16.0))
                    .bg(t.primary)
                    .px(px(16.0))
                    .py(px(8.0))
                    .text_sm_tw()
                    .text_color(t.primary_foreground)
                    .child(message.to_string()),
            )
            .child(user_avatar(&user, px(36.0)).text_xs_tw());
        let part = Part::User(i);
        if self.st.animating.contains(&part) { enter_animation(row, part, 600, 8.0) } else { row.into_any_element() }
    }

    /// The reply of turn `i`: avatar rail, tool activity, the (possibly partly revealed) reply and the copy button.
    /// A finished turn without reply text or tool calls shows nothing.
    fn assistant_turn(
        &self,
        i: usize,
        turn: &Turn,
        is_active: bool,
        is_revealing: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let full = turn.assistant.as_str();
        if !is_active && full.trim().is_empty() && turn.tools.is_empty() {
            return None;
        }
        let t = cx.tokens();
        let visible = if is_active && is_revealing { self.reveal.displayed() } else { full };
        let has_running_tool = turn.tools.iter().any(|c| c.status == ToolStatus::Running);
        let group: SharedString = format!("turn-{i}").into();

        let rail = div()
            .flex()
            .flex_col()
            .items_center()
            .flex_shrink_0()
            .w(px(36.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(36.0))
                    .overflow_hidden()
                    .rounded_full()
                    .bg(alpha(t.primary, 0.1))
                    .child(img(images::SUPPORT_DARU_ALT_SM).size(px(36.0)).rounded_full().object_fit(ObjectFit::Cover)),
            )
            .child(div().mt(px(8.0)).w(px(1.0)).flex_1().bg(alpha(t.border, 0.6)));

        let mut col = div().flex().flex_col().flex_1().min_w_0().gap(px(8.0)).pb(px(8.0));
        if let Some(tools) = self.tool_activity(i, &turn.tools, is_active, cx) {
            col = col.child(tools);
        }
        if is_active && visible.trim().is_empty() && !has_running_tool {
            col = col.child(loading_indicator(cx));
        }
        if !visible.trim().is_empty() {
            col = col.child(
                Markdown::new(SharedString::from(format!("chat-msg-{i}")), visible.to_string())
                    .style(MarkdownStyle::Chat)
                    .w_full(),
            );
        }
        if !is_active && !full.trim().is_empty() {
            col = col.child(Self::message_actions(i, full));
        }

        let row = div().group(group).flex().gap(px(12.0)).child(rail).child(col);
        let part = Part::Reply(i);
        if !self.st.animating.contains(&part) {
            return Some(row.into_any_element());
        }
        Some(enter_animation(row, part, 600, 8.0))
    }

    /// The tool calls of a turn: pills while the turn is active, otherwise behind a "Used N tools" toggle.
    fn tool_activity(
        &self,
        turn: usize,
        tool_calls: &[ToolCall],
        is_turn_active: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if tool_calls.is_empty() {
            return None;
        }
        let t = cx.tokens();
        let pills = div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(6.0))
            .children(tool_calls.iter().enumerate().map(|(j, call)| self.tool_pill(Part::Tool(turn, j), call, cx)));
        if is_turn_active {
            return Some(pills.into_any_element());
        }

        let open = self.expanded_tools.contains(&turn);
        let n = tool_calls.len();
        let (muted, fg) = (t.muted_foreground, t.foreground);
        let trigger = div()
            .id(SharedString::from(format!("tools-toggle-{turn}")))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded_full()
            .border_1()
            .border_color(alpha(t.border, 0.6))
            .bg(alpha(t.muted, 0.3))
            .px(px(10.0))
            .py(px(4.0))
            .text_xs_tw()
            .text_color(muted)
            .cursor_pointer()
            .hover(move |s| s.text_color(fg).text_xs_tw())
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.expanded_tools.remove(&turn) {
                    this.expanded_tools.insert(turn);
                }
                cx.notify();
            }))
            .child(Icon::new(icons::WRENCH).size(px(14.0)))
            .child(format!("Used {n} {}", if n == 1 { "tool" } else { "tools" }))
            .child(
                Icon::new(icons::CHEVRON_DOWN)
                    .size(px(12.0))
                    .when(open, |i| i.transform(Transformation::rotate(percentage(0.5)))),
            );
        Some(
            div()
                .flex()
                .flex_col()
                .child(div().flex().child(trigger))
                .when(open, |d| d.child(div().pt(px(6.0)).child(pills)))
                .into_any_element(),
        )
    }

    /// One tool call as a pill: icon, description, and a spinner while running or a check mark when done.
    fn tool_pill(&self, part: Part, call: &ToolCall, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let meta = tool_meta(&call.action);
        let running = call.status == ToolStatus::Running;
        let pill = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .max_w_full()
            .rounded_full()
            .border_1()
            .px(px(10.0))
            .py(px(4.0))
            .text_xs_tw()
            .when(running, |d| {
                d.border_color(alpha(t.primary, 0.3)).bg(alpha(t.primary, 0.05)).text_color(t.foreground)
            })
            .when(!running, |d| {
                d.border_color(alpha(t.border, 0.6)).bg(alpha(t.muted, 0.4)).text_color(t.muted_foreground)
            })
            .child(Icon::new(meta.icon).size(px(14.0)).text_color(alpha(t.primary, 0.8)))
            .child(div().overflow_hidden().text_ellipsis().whitespace_nowrap().child(if running {
                meta.running
            } else {
                meta.done
            }))
            .child(if running {
                spinner().with_size(px(12.0)).color(t.primary).into_any_element()
            } else {
                Icon::new(icons::CHECK).size(px(12.0)).text_color(alpha(t.primary, 0.7)).into_any_element()
            });
        if self.st.animating.contains(&part) { enter_animation(pill, part, 500, 4.0) } else { pill.into_any_element() }
    }

    /// Copy button for a finished reply. Shown once the turn is no longer streaming or revealing.
    fn message_actions(turn: usize, content: &str) -> AnyElement {
        let button = Clipboard::new(SharedString::from(format!("copy-{turn}"))).value(content.to_string()).small();
        div().flex().items_center().gap(px(4.0)).child(button).into_any_element()
    }
}

/// Placeholder shown before the first message: the Daru picture and a welcome text.
fn empty_state(window: &mut Window, cx: &mut Context<AiChatDialog>) -> impl IntoElement {
    let t = cx.tokens();
    // Image size: 35% of the window height, kept between 160 and 280 px.
    let size = (window.viewport_size().height * 0.35).min(px(280.0)).max(px(160.0));
    EmptyState::picture(
        img(images::SUPPORT_DARU).size(size).object_fit(ObjectFit::Contain),
        "Welcome to Daru's Help Desk!",
    )
    .description(
        "I'm your friendly Daru assistant, ready to help with all your addon needs. Feel free to ask me anything!",
    )
    .flex_1()
    .text_color(t.primary)
}

/// "Daru is thinking" label followed by three bouncing dots.
fn loading_indicator(cx: &mut Context<AiChatDialog>) -> impl IntoElement {
    let t = cx.tokens();
    let dot = |i: usize, delay: f32| {
        div().size(px(6.0)).rounded_full().bg(t.primary).relative().with_animation(
            SharedString::from(format!("dot-{i}")),
            Animation::new(Duration::from_millis(1000)).repeat(),
            move |d, delta| {
                // One bounce per second: the dot starts 1.5 px up, rests at the halfway point and rises back,
                // with cubic easing.
                let p = (delta + 1.0 - delay).fract();
                let lift = if p < 0.5 {
                    let s = p * 2.0;
                    1.0 - s * s * s
                } else {
                    let s = (p - 0.5) * 2.0;
                    1.0 - (1.0 - s).powi(3)
                };
                d.top(px(-1.5 * lift))
            },
        )
    };
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(4.0))
        .child(div().mr(px(4.0)).text_xs_tw().text_color(alpha(t.muted_foreground, 0.7)).child("Daru is thinking"))
        .child(dot(0, 0.0))
        .child(dot(1, 0.1))
        .child(dot(2, 0.2))
}
