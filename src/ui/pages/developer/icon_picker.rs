//! The "Addon icon" dialog: drop zone, preview and Remove, Cancel and Save buttons.
use std::{path::PathBuf, rc::Rc};

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, ExternalPaths, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, KeyDownEvent, ObjectFit, ParentElement, PathPromptOptions, Render, StatefulInteractiveElement, Styled,
    StyledImage, Window, div, img, prelude::FluentBuilder as _, px,
};

use super::api;
use crate::{
    state::run_blocking,
    ui::{
        components::{Button, ButtonSize, ButtonVariant, DialogFooter, DialogSpec, dialog, markdown_http},
        icons,
        theme::{TokensExt, TwText, alpha, radius},
    },
};

/// Result of the picker's Save.
#[derive(Clone, Debug, PartialEq)]
pub enum IconChange {
    /// "Remove" was saved.
    Removed,
    /// A PNG was uploaded.
    Uploaded { asset_id: String, url: String },
}

type OnSaved = Rc<dyn Fn(IconChange, &mut Window, &mut App)>;

/// Opens the dialog. `current_icon_url` is the addon's current icon (preview + enables "Remove"); `on_saved` runs after
/// the dialog closed, with the committed change.
pub fn open(
    window: &mut Window,
    cx: &mut App,
    current_icon_url: Option<String>,
    on_saved: impl Fn(IconChange, &mut Window, &mut App) + 'static,
) {
    let on_saved: OnSaved = Rc::new(on_saved);
    let form = cx.new(|cx| IconPickerForm::new(current_icon_url, on_saved, cx));
    let saving = form.clone();
    dialog::open(
        window,
        cx,
        DialogSpec::new()
            .title("Addon icon")
            .description("Shown in search results, the detail header, and installed lists.")
            .max_width(px(576.0))
            // X, Escape and overlay clicks are ignored while saving
            .locked(move |cx| saving.read(cx).saving)
            .content_view(form),
    );
}

/// A validated, staged PNG.
struct Staged {
    path: PathBuf,
    name: String,
    size: u64,
}

/// Size in whole kilobytes, rounded up, for the staged-file caption.
fn kb_label(size: u64) -> String {
    format!("{} KB", size.div_ceil(1024))
}

/// The caption under the previews.
fn status_line(staged: Option<&Staged>, remove: bool, has_current: bool) -> String {
    if let Some(s) = staged {
        format!("{} · {}", s.name, kb_label(s.size))
    } else if remove {
        "Icon will be removed".into()
    } else if has_current {
        "Current icon".into()
    } else {
        "No image selected".into()
    }
}

struct IconPickerForm {
    current_icon_url: Option<String>,
    staged: Option<Staged>,
    remove: bool,
    errors: Vec<String>,
    validating: bool,
    saving: bool,
    /// Bumped by every stage / remove so late validation results are dropped.
    session: u64,
    focus: FocusHandle,
    on_saved: OnSaved,
}

impl IconPickerForm {
    fn new(current_icon_url: Option<String>, on_saved: OnSaved, cx: &mut Context<Self>) -> Self {
        Self {
            current_icon_url,
            staged: None,
            remove: false,
            errors: Vec::new(),
            validating: false,
            saving: false,
            session: 0,
            focus: cx.focus_handle(),
            on_saved,
        }
    }

    fn busy(&self) -> bool {
        self.validating || self.saving
    }

    fn has_pending_change(&self) -> bool {
        self.staged.is_some() || self.remove
    }

    /// Opens the native file picker.
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        let picked =
            cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: None });
        cx.spawn_in(window, async move |this, cx| {
            // cancelled or no portal: nothing to stage
            let Ok(Ok(Some(paths))) = picked.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let _ = this.update_in(cx, |this, window, cx| this.stage(path, window, cx));
        })
        .detach();
    }

    /// Validates the file off the UI thread, then stages it or shows the problems.
    fn stage(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        self.session += 1;
        let session = self.session;
        self.validating = true;
        self.staged = None;
        self.remove = false;
        self.errors.clear();
        cx.notify();
        let work = {
            let path = path.clone();
            run_blocking(cx, move || {
                let problems = api::validate_icon_file(&path);
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                (problems, size)
            })
        };
        cx.spawn_in(window, async move |this, cx| {
            let (problems, size) = work.await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.session != session {
                    return;
                }
                this.validating = false;
                if problems.is_empty() {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    this.staged = Some(Staged { path, name, size });
                } else {
                    this.errors = problems;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn mark_removed(&mut self, cx: &mut Context<Self>) {
        self.session += 1;
        self.staged = None;
        self.remove = true;
        self.errors.clear();
        cx.notify();
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_pending_change() || self.busy() {
            return;
        }
        let on_saved = self.on_saved.clone();
        let staged_path = if self.remove { None } else { self.staged.as_ref().map(|s| s.path.clone()) };
        let Some(path) = staged_path else {
            // remove: nothing to upload
            dialog::close(window, cx);
            on_saved(IconChange::Removed, window, cx);
            return;
        };
        self.saving = true;
        cx.notify();
        let work = run_blocking(cx, move || api::upload_icon(&path));
        cx.spawn_in(window, async move |this, cx| {
            let result = work.await;
            let change = this
                .update_in(cx, |this, _, cx| {
                    this.saving = false;
                    cx.notify();
                    match result {
                        Ok(icon) => Some(IconChange::Uploaded { asset_id: icon.asset_id, url: icon.url }),
                        Err(e) => {
                            this.errors = vec![e.message()];
                            None
                        }
                    }
                })
                .ok()
                .flatten();
            if let Some(change) = change {
                let _ = cx.update(|window, cx| {
                    dialog::close(window, cx);
                    on_saved(change, window, cx);
                });
            }
        })
        .detach();
    }

    /// Preview of the placeholder, the staged file or the current remote icon, `edge` px square.
    fn preview(&self, edge: f32, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let frame = div().flex().flex_none().size(px(edge)).rounded(radius::LG).border_1().border_color(t.border);
        let image = |source: gpui_kit::ImageSource| {
            img(source).size(px(edge - 2.0)).rounded(radius::LG - px(1.0)).object_fit(ObjectFit::Cover)
        };
        if !self.remove
            && let Some(staged) = &self.staged
        {
            return frame.child(image(staged.path.clone().into())).into_any_element();
        }
        match self.current_icon_url.as_ref().filter(|_| !self.remove) {
            Some(url) => {
                markdown_http::install(cx);
                frame.child(image(url.clone().into())).into_any_element()
            }
            None => frame
                .items_center()
                .justify_center()
                .bg(t.card)
                .text_color(t.primary)
                .child(Icon::new(icons::IMAGE_PLUS).size(px(edge / 2.0)))
                .into_any_element(),
        }
    }

    fn drop_zone(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = cx.tokens();
        let busy = self.busy();
        let primary = t.primary;
        let mut zone = div()
            .id("icon-drop-zone")
            .flex_1()
            .min_w_0()
            .min_h(px(160.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .p_4()
            .cursor_pointer()
            .rounded(radius::LG)
            .border_dashed()
            .border_color(if self.errors.is_empty() { t.border } else { t.destructive })
            .hover(move |s| s.border_color(primary).bg(alpha(primary, 0.05)))
            .drag_over::<ExternalPaths>(
                move |s, _, _, _| if busy { s } else { s.border_color(primary).bg(alpha(primary, 0.1)) },
            )
            .track_focus(&self.focus)
            .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx)))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    this.browse(window, cx);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(path) = paths.paths().first() {
                    this.stage(path.clone(), window, cx);
                }
            }));
        // gpui has no helper for a 1.5 px border, so the widths are set directly.
        let style = zone.style();
        style.border_widths.top = Some(px(1.5).into());
        style.border_widths.right = Some(px(1.5).into());
        style.border_widths.bottom = Some(px(1.5).into());
        style.border_widths.left = Some(px(1.5).into());
        zone.child(Icon::new(icons::IMAGE_PLUS).size(px(20.0)).text_color(t.primary))
            .child(div().text_sm_tw().font_weight(FontWeight::SEMIBOLD).child("Drop image or click to browse"))
            .child(div().text_xs_tw().text_color(t.muted_foreground).child("Square PNG · min 50×50 · max 3 MB"))
    }

    fn preview_card(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = cx.tokens();
        let muted = t.muted_foreground;
        let border = t.border;
        let status = status_line(self.staged.as_ref(), self.remove, self.current_icon_url.is_some());
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(168.0))
            .items_center()
            .gap(px(10.0))
            .p_3()
            .rounded(radius::LG)
            .border_1()
            .border_color(border)
            .child(div().text_size(px(10.0)).line_height(px(15.0)).text_color(muted).child("PREVIEW"))
            .child(div().flex().items_end().gap_3().child(self.preview(96.0, cx)).child(self.preview(40.0, cx)))
            .child(
                div()
                    .text_center()
                    .text_size(px(10.0))
                    .line_height(px(15.0))
                    .text_color(muted)
                    .child("96px · 40px preview sizes"),
            )
            .child(div().text_center().text_size(px(11.0)).line_height(px(16.5)).text_color(muted).child(status))
    }
}

impl Render for IconPickerForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let busy = self.busy();
        let saving = self.saving;
        let can_remove = !busy && (self.has_pending_change() || self.current_icon_url.is_some());
        let can_save = self.has_pending_change() && !busy;

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().flex().gap_4().child(self.drop_zone(cx)).child(self.preview_card(cx)))
            .when(!self.errors.is_empty(), |d| {
                d.child(self.errors.iter().fold(div().flex().flex_col().gap_1(), |list, message| {
                    list.child(
                        div()
                            .text_xs_tw()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.destructive)
                            .child(message.clone()),
                    )
                }))
            })
            .child(
                DialogFooter::new()
                    .child(
                        Button::new("icon-remove")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .icon(icons::TRASH_2)
                            .label("Remove")
                            .disabled(!can_remove)
                            .on_click(cx.listener(|this, _, _, cx| this.mark_removed(cx))),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("icon-cancel")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .label("Cancel")
                                    .disabled(saving)
                                    .on_click(|_, window, cx| dialog::close(window, cx)),
                            )
                            .child(
                                Button::new("icon-save")
                                    .size(ButtonSize::Sm)
                                    .label(if saving { "Uploading…" } else { "Save" })
                                    .disabled(!can_save)
                                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kb_label_rounds_up() {
        assert_eq!(kb_label(0), "0 KB");
        assert_eq!(kb_label(1), "1 KB");
        assert_eq!(kb_label(1024), "1 KB");
        assert_eq!(kb_label(1025), "2 KB");
        assert_eq!(kb_label(3 * 1024 * 1024), "3072 KB");
    }

    #[test]
    fn status_line_priority() {
        let staged = Staged { path: PathBuf::from("/x/icon.png"), name: "icon.png".into(), size: 2049 };
        assert_eq!(status_line(Some(&staged), false, true), "icon.png · 3 KB");
        assert_eq!(status_line(None, true, true), "Icon will be removed");
        assert_eq!(status_line(None, false, true), "Current icon");
        assert_eq!(status_line(None, false, false), "No image selected");
    }
}
