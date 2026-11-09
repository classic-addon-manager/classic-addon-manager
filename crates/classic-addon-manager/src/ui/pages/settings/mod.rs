//! Settings page: documents directory override, accent colour picker and shortcuts to the cache and data folders.
use std::{path::PathBuf, time::Duration};

use cam_core::{file, paths::Paths};
use cam_ui::toast;
use cam_ui::{
    components::*,
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    Animation, AnimationExt as _, ClickEvent, Context, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement, PathPromptOptions, Render, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    ease_in_out, hsla, prelude::FluentBuilder as _, px,
};

use crate::state::{self, SettingsState, tasks::error_message};
use crate::ui::accent::ACCENT_PRESETS;

const DIALOG_TITLE: &str = "Select ArcheAge Classic Documents directory";
const ERROR_TITLE: &str = "Error during directory selection.";

type DirFn = fn(&Paths) -> anyhow::Result<PathBuf>;

pub struct SettingsPage {
    settings: Entity<SettingsState>,
    err_docs_path: String,
}

impl SettingsPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = state::settings(cx);
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        cx.observe(&state::preferences(cx), |_, _, cx| cx.notify()).detach();
        settings.update(cx, |s, cx| s.load_config(cx));
        Self { settings, err_docs_path: String::new() }
    }

    /// Opens the folder picker, then checks the chosen folder and stores it as the ArcheAge Classic documents path
    /// with `Config::set_aac_path_checked`.
    fn open_select(&mut self, cx: &mut Context<Self>) {
        self.err_docs_path.clear();
        cx.notify();
        let backend = state::backend(cx);
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(DIALOG_TITLE.into()),
        });
        cx.spawn(async move |this, cx| {
            let picked = match rx.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(e)) => {
                    let msg = state::tasks::sentence_case(&e.to_string());
                    let _ = this.update(cx, |t, cx| {
                        t.err_docs_path = msg;
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(path) = picked else { return };
            let path = path.to_string_lossy().to_string();
            let checked = path.clone();
            let res = state::run_blocking(cx, move || {
                backend.config.set_aac_path_checked(&checked)?;
                // The path is set either way, the next install retries the file.
                if let Err(e) = backend.addon_dir().and_then(|dir| dir.ensure_addons_txt_exists()) {
                    log::error!("Could not create addons.txt: {e:#}");
                }
                anyhow::Ok(())
            })
            .await;
            let _ = this.update(cx, |t, cx| match res {
                Ok(()) => {
                    t.settings.update(cx, |s, cx| s.set_aac_path(path.clone(), cx));
                    toast::success_titled(cx, "Success", format!("Path set to: {path}"));
                }
                Err(e) => {
                    log::error!("Could not set the AAC documents path: {e:#}");
                    t.err_docs_path = error_message(&e);
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

pub(crate) fn page_header(
    t: &cam_ui::theme::Tokens,
    icon: icons::Lucide,
    title: &'static str,
    subtitle: &'static str,
) -> gpui_kit::Div {
    div().flex_none().border_b_1().border_color(t.border).bg(alpha(t.background, 0.95)).child(
        div().h(px(64.0)).w_full().px_4().flex().items_center().gap_4().child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .rounded(radius::LG)
                        .bg(alpha(t.primary, 0.1))
                        .p_2()
                        .child(Icon::new(icon).size(px(24.0)).text_color(t.primary)),
                )
                .child(
                    div()
                        .child(div().text_xl_tw().font_weight(FontWeight::SEMIBOLD).child(title))
                        .child(div().text_sm_tw().text_color(t.muted_foreground).child(subtitle)),
                ),
        ),
    )
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let s = self.settings.read(cx);
        let (auto, aac_path, initialized, is_loading, load_error) =
            (s.auto_path_detection, s.aac_path.clone(), s.is_initialized, s.is_loading, s.load_error.clone());

        let load_error_box = load_error.clone().map(|err| {
            error_box(&t, "Failed to load settings", err.clone())
                .mx_4()
                .mt_4()
                .items_center()
                .child(div().flex_1())
                .child(
                    Button::new("settings-retry")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .label("Retry")
                        .disabled(is_loading)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.settings.update(cx, |s, cx| s.load_config(cx));
                        })),
                )
        });

        let override_row = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(24.0))
            .px_4()
            .py(px(14.0))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm_tw()
                            .line_height(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Override automatic detection"),
                    )
                    .child(
                        div()
                            .text_xs_tw()
                            .text_color(t.muted_foreground)
                            .child("Specify the documents directory yourself"),
                    ),
            )
            .child(Switch::new("override-switch").checked(!auto).disabled(!initialized).on_change(cx.listener(
                |this, checked: &bool, _, cx| {
                    let enabled = !*checked;
                    this.settings.update(cx, |s, cx| s.set_auto_path_detection(enabled, cx));
                    this.err_docs_path.clear();
                    cx.notify();
                },
            )));

        let border60 = alpha(t.border, 0.6);
        let bottom: gpui_kit::AnyElement = if !initialized {
            if load_error.is_some() {
                div().into_any_element()
            } else {
                div()
                    .border_t_1()
                    .border_color(border60)
                    .px_4()
                    .py_3()
                    .child(div().text_xs_tw().text_color(t.muted_foreground).child("Loading settings…"))
                    .into_any_element()
            }
        } else if !auto {
            div()
                .flex()
                .flex_col()
                .gap_3()
                .border_t_1()
                .border_color(border60)
                .bg(alpha(t.muted, 0.15))
                .px_4()
                .py_4()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .h(px(36.0))
                                .flex()
                                .items_center()
                                .px_3()
                                .rounded(radius::MD)
                                .border_1()
                                .border_color(t.input)
                                .bg(alpha(t.input, 0.3))
                                .opacity(0.5)
                                .overflow_hidden()
                                .text_xs_tw()
                                .font_mono_tw()
                                .when(aac_path.is_empty(), |d| {
                                    d.text_color(t.muted_foreground).child(if cfg!(windows) {
                                        "C:\\AAClassic\\Documents\\..."
                                    } else {
                                        "~/.local/share/AAClassic/.../Documents/AAClassic"
                                    })
                                })
                                .when(!aac_path.is_empty(), |d| d.child(aac_path.clone())),
                        )
                        .child(
                            Button::new("browse")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Icon)
                                .icon(icons::FOLDER_OPEN)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_select(cx))),
                        ),
                )
                .when(!self.err_docs_path.is_empty(), |d| {
                    d.child(error_box(&t, ERROR_TITLE, self.err_docs_path.clone()))
                })
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .text_xs_tw()
                        .line_height(px(19.5))
                        .text_color(t.muted_foreground)
                        .child("Directory should contain an\u{00a0}")
                        .child(div().text_color(alpha(t.foreground, 0.8)).child("Addon"))
                        .child("\u{00a0}folder and\u{00a0}")
                        .child(div().text_color(alpha(t.foreground, 0.8)).child("system.cfg"))
                        .child("."),
                )
                .into_any_element()
        } else {
            let ping = div().absolute().size_full().rounded_full().bg(alpha(tw(Tw::Emerald, 400), 0.4)).with_animation(
                "detect-ping",
                Animation::new(Duration::from_secs(1)).repeat().with_easing(ease_in_out),
                |d, delta| d.opacity(0.75 * (1.0 - delta)),
            );
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .border_t_1()
                .border_color(border60)
                .px_4()
                .py_3()
                .child(
                    div()
                        .relative()
                        .flex_none()
                        .size(px(8.0))
                        .child(ping)
                        .child(div().relative().size(px(8.0)).rounded_full().bg(tw(Tw::Emerald, 500))),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .text_xs_tw()
                        .text_color(t.muted_foreground)
                        .child("Detecting documents directory automatically")
                        .when(!aac_path.is_empty(), |d| {
                            d.child(" · ").child(
                                div().font_mono_tw().text_color(alpha(t.foreground, 0.7)).child(aac_path.clone()),
                            )
                        }),
                )
                .into_any_element()
        };

        let accent = state::preferences(cx).read(cx).accent_color;
        let swatches = ACCENT_PRESETS.iter().map(|preset| {
            let swatch = preset.accent.color();
            let selected = preset.id == accent;
            let id = preset.id;
            div()
                .id(SharedString::from(format!("accent-{}", preset.label)))
                .flex()
                .items_center()
                .justify_center()
                .relative()
                .size(px(32.0))
                .rounded_full()
                .bg(swatch)
                .cursor_pointer()
                .hover(|s| s.opacity(0.9))
                .on_click(move |_, _, cx| {
                    state::preferences(cx).update(cx, |p, cx| p.set_accent_color(id, cx));
                })
                .when(!selected, |d| d.border_1().border_color(hsla(0., 0., 0., 0.25)))
                .when(selected, |d| {
                    let ring = |inset: f32, c| {
                        div()
                            .absolute()
                            .top(px(-inset))
                            .left(px(-inset))
                            .size(px(32.0 + 2.0 * inset))
                            .rounded_full()
                            .bg(c)
                    };
                    d.child(ring(4.0, swatch))
                        .child(ring(2.0, t.card))
                        .child(div().absolute().size_full().rounded_full().bg(swatch))
                        .child(Icon::new(icons::CHECK).size(px(16.0)).text_color(t.primary_foreground))
                })
        });

        let actions: [(&'static str, &'static str, &'static str, icons::Lucide, DirFn); 2] = [
            (
                "cache",
                "Cache location",
                "Temporary files and downloaded addon packages",
                icons::FOLDER_OPEN,
                Paths::cache_dir,
            ),
            ("data", "Data location", "Application config and persistent data", icons::DATABASE, Paths::data_dir),
        ];
        let n = actions.len();
        let quick = actions.into_iter().enumerate().map(|(i, (id, label, description, icon, dir))| {
            div()
                .id(SharedString::from(format!("loc-{id}")))
                .group("loc-row")
                .flex()
                .w_full()
                .items_center()
                .gap_3()
                .px_4()
                .py(px(14.0))
                .cursor_pointer()
                .when(i > 0, |d| d.border_t_1().border_color(border60))
                .when(i == 0, |d| d.rounded_t(radius::XL))
                .when(i == n - 1, |d| d.rounded_b(radius::XL))
                .hover(|s| s.bg(alpha(t.muted, 0.4)))
                .on_click(move |_, _, cx| {
                    let backend = state::backend(cx);
                    cx.spawn(async move |cx| {
                        if let Err(e) =
                            state::run_blocking(cx, move || file::open_directory(dir(&backend.paths)?)).await
                        {
                            let msg = error_message(&e);
                            let msg = if msg.is_empty() { format!("Failed to open {label}") } else { msg };
                            cx.update(|cx| toast::show(cx, toast::Toast::new("Error", msg)));
                        }
                    })
                    .detach();
                })
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(36.0))
                        .rounded(radius::LG)
                        .bg(alpha(t.muted, 0.6))
                        .text_color(t.muted_foreground)
                        .group_hover("loc-row", |s| s.bg(alpha(t.primary, 0.1)).text_color(t.primary))
                        .child(Icon::new(icon).size(px(16.0))),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(div().text_sm_tw().line_height(px(14.0)).font_weight(FontWeight::MEDIUM).child(label))
                        .child(div().mt_1().truncate().text_xs_tw().text_color(t.muted_foreground).child(description)),
                )
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap_1()
                        .text_xs_tw()
                        .text_color(t.muted_foreground)
                        .group_hover("loc-row", |s| s.text_color(t.foreground))
                        .child("Open")
                        .child(Icon::new(icons::CHEVRON_RIGHT).size(px(14.0)).opacity(0.6)),
                )
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(page_header(&t, icons::SETTINGS_2, "Settings", "Manage Classic Addon Manager preferences"))
            .child(
                ScrollArea::new("settings-scroll").flex_1().min_h_0().child(
                    div()
                        .w_full()
                        .max_w(px(672.0))
                        .mx_auto()
                        .px_4()
                        .py_8()
                        .flex()
                        .flex_col()
                        .gap_8()
                        .child(
                            Section::new("Documents directory")
                                .description("Where the addon manager looks for your ArcheAge Classic documents")
                                .children(load_error_box)
                                .child(override_row)
                                .child(bottom),
                        )
                        .child(
                            Section::new("Accent color")
                                .description("Choose the highlight color used across the app")
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap_3()
                                        .px_4()
                                        .py(px(14.0))
                                        .children(swatches),
                                ),
                        )
                        .child(
                            Section::new("Locations")
                                .description("Open folders used by Classic Addon Manager")
                                .child(div().children(quick)),
                        ),
                ),
            )
    }
}
