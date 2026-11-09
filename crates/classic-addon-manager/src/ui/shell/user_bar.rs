//! Sidebar account area: a "Sign in with Discord" button, or the account dropdown plus the chat button once signed in.
use cam_ui::{
    components::{Align, DialogSpec, DropdownMenu, MenuItem, Side, Tooltip, dialog},
    icons,
    theme::{TokensExt, TwText, alpha, radius},
    toast,
};
use gpui_kit::component::Icon;
use gpui_kit::{
    AppContext as _, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, Rgba,
    StatefulInteractiveElement, Styled, Transformation, Window, div, percentage, prelude::FluentBuilder as _, px,
};

use crate::{
    state,
    ui::chat::{AiChatDialog, user_avatar},
};

const DISCORD_AUTH_URL: &str = "https://discord.com/oauth2/authorize?client_id=1331010099916836914&response_type=code&redirect_uri=https%3A%2F%2Faac.gaijin.dev%2Fauth%2Fdiscord%2Fcallback2&scope=identify";

pub struct UserBar {
    /// The chat view lives as long as the signed-in session, so its messages survive closing and reopening the dialog.
    chat: Option<Entity<AiChatDialog>>,
    menu_open: bool,
}

impl UserBar {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let user = state::user(cx);
        cx.observe(&user, |_, _, cx| cx.notify()).detach();
        Self { chat: None, menu_open: false }
    }

    fn open_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chat = match &self.chat {
            Some(c) => c.clone(),
            None => {
                let c = cx.new(|cx| AiChatDialog::new(window, cx));
                self.chat = Some(c.clone());
                c
            }
        };
        let on_close = chat.clone();
        dialog::open(
            window,
            cx,
            // Chat dialog without padding or a close button, up to 70% of the window wide.
            DialogSpec::new()
                .padded(false)
                .show_close(false)
                .max_width(px(100_000.0))
                .max_width_frac(0.7)
                .content_view(chat.clone())
                .on_close(move |_, cx| on_close.update(cx, |c, cx| c.on_closed(cx))),
        );
        chat.update(cx, |c, cx| c.on_open(window, cx));
    }

    fn sign_out(cx: &mut gpui_kit::App) {
        let sign_out = state::user(cx).update(cx, |u, cx| u.sign_out(cx));
        cx.spawn(async move |cx| {
            if let Err(e) = sign_out.await {
                log::error!("Sign out failed: {e:#}");
                cx.update(|cx| toast::error(cx, "Failed to sign out. Session persists."));
            }
        })
        .detach();
    }
}

impl Render for UserBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let (authenticated, user) = {
            let u = state::user(cx).read(cx);
            (u.is_authenticated(), u.user.clone())
        };

        if !authenticated {
            // Signing out discards the chat view.
            self.chat = None;
            self.menu_open = false;
            let discord: gpui_kit::Hsla = Rgba { r: 88.0 / 255.0, g: 101.0 / 255.0, b: 242.0 / 255.0, a: 1.0 }.into();
            return div()
                .w_full()
                .px(px(12.0))
                .pb(px(4.0))
                .child(
                    div()
                        .id("discord-sign-in")
                        .flex()
                        .w_full()
                        .items_center()
                        .justify_center()
                        .gap(px(10.0))
                        .rounded(radius::LG)
                        .px(px(12.0))
                        .py(px(10.0))
                        .bg(alpha(discord, 0.9))
                        .text_color(gpui_kit::white())
                        .cursor_pointer()
                        .hover(move |s| s.bg(discord))
                        .on_click(|_, _, cx| cx.open_url(DISCORD_AUTH_URL))
                        .child(Icon::new(icons::DISCORD).size(px(16.0)).flex_shrink_0())
                        .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child("Sign in with Discord")),
                )
                .into_any_element();
        }

        let hover_bg = alpha(t.secondary, 0.6);
        let this = cx.entity();
        let menu_open = self.menu_open;

        let trigger = div()
            .id("user-menu-trigger")
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .gap(px(10.0))
            .rounded(radius::LG)
            .px(px(8.0))
            .py(px(6.0))
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg))
            .child(user_avatar(&user, px(32.0)).text_size(px(10.0)).font_weight(FontWeight::MEDIUM))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(14.0))
                            .line_height(px(17.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.foreground)
                            .child(user.username.clone()),
                    )
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(11.0))
                            .line_height(px(13.75))
                            .text_color(t.muted_foreground)
                            .child("Account"),
                    ),
            )
            .child(
                Icon::new(icons::CHEVRON_UP)
                    .size(px(14.0))
                    .flex_shrink_0()
                    .text_color(alpha(t.muted_foreground, 0.7))
                    .when(menu_open, |i| i.transform(Transformation::rotate(percentage(0.5)))),
            );

        let username = user.username.clone();
        let dropdown = DropdownMenu::new()
            .id("user-menu")
            .trigger(trigger)
            .side(Side::Top)
            .align(Align::Start)
            .side_offset(8.0)
            .open(menu_open)
            .on_open_change({
                let this = this.clone();
                move |open, _, cx| {
                    this.update(cx, |t, cx| {
                        t.menu_open = open;
                        cx.notify();
                    })
                }
            })
            .menu(move |m| {
                m.width(px(208.0)).label_with_subtitle(username.clone(), "Signed in with Discord").separator().item(
                    // Keep the menu open while signing out.
                    MenuItem::new("Sign out")
                        .destructive()
                        .icon(icons::LOG_OUT)
                        .keep_open(true)
                        .on_click(|_, cx| UserBar::sign_out(cx)),
                )
            })
            .flex_1()
            .min_w_0();

        let chat_button = Tooltip::new("Ask a friendly Daru").side(Side::Top).side_offset(6.0).child(
            div()
                .id("user-chat-button")
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size(px(36.0))
                .rounded(radius::LG)
                .text_color(t.muted_foreground)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).text_color(t.primary))
                .on_click(cx.listener(|this, _, window, cx| this.open_chat(window, cx)))
                .child(Icon::new(icons::MESSAGE_CIRCLE_QUESTION_MARK).size(px(16.0))),
        );

        div()
            .w_full()
            .px(px(12.0))
            .pb(px(4.0))
            .child(
                div()
                    .rounded(radius::XL)
                    .border_1()
                    .border_color(alpha(t.border, 0.5))
                    .bg(alpha(t.secondary, 0.2))
                    .p(px(6.0))
                    .child(div().flex().items_center().gap(px(4.0)).child(dropdown).child(chat_button)),
            )
            .into_any_element()
    }
}
