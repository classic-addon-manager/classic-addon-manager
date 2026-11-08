//! Developer workspace: the user's published addons and submissions in a grouped list, a detail pane for the selected
//! entry, and the form for publishing a new addon.
mod api;
mod bits;
mod cta;
mod details;
mod dialogs;
mod edit_panel;
mod fields;
mod fmt;
mod form;
mod host;
mod icon_picker;
mod owned;
mod parse;
mod publish_form;
mod queries;
mod review_status;
mod statistics;
mod submission_details;
mod toolbar;
mod types;

use std::{collections::HashSet, time::Duration};

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyView, AppContext as _, Context, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use self::{
    bits::loading_block,
    details::AddonDetails,
    host::Host,
    owned::{OwnedAddonsData, OwnedSubmission},
    publish_form::{PublishAddonForm, PublishFormEvent},
    queries::DevQueries,
    submission_details::SubmissionDetails,
    toolbar::{DeveloperScope, DeveloperToolbar},
};
use crate::{
    state,
    ui::{
        components::{AddonIcon, Button, ScrollArea, addon_icon_url},
        icons,
        theme::{TokensExt, TwText, alpha, radius},
        titlebar,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    List,
    Form,
}

/// The list entry the detail pane shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Selection {
    /// A published addon, by name.
    Addon(String),
    Submission(i64),
}

impl Selection {
    fn element_id(&self) -> ElementId {
        let name = match self {
            Selection::Addon(name) => format!("addon:{name}"),
            Selection::Submission(id) => format!("submission:{id}"),
        };
        SharedString::from(name).into()
    }
}

/// The detail pane's view, with the entry it was built for.
enum Detail {
    Addon(String, Entity<AddonDetails>),
    Submission(i64, Entity<SubmissionDetails>),
}

impl Detail {
    fn shows(&self, selection: &Selection) -> bool {
        match (self, selection) {
            (Detail::Addon(name, _), Selection::Addon(selected)) => name == selected,
            (Detail::Submission(id, _), Selection::Submission(selected)) => id == selected,
            _ => false,
        }
    }

    fn view(&self) -> AnyView {
        match self {
            Detail::Addon(_, v) => v.clone().into(),
            Detail::Submission(_, v) => v.clone().into(),
        }
    }
}

pub struct DeveloperPage {
    view: View,
    selection: Option<Selection>,
    pub(crate) scope: DeveloperScope,
    queries: Entity<DevQueries>,
    toolbar: Entity<DeveloperToolbar>,
    publish_form: Option<Entity<PublishAddonForm>>,
    detail: Option<Detail>,
    slot_shown: bool,
}

const POLL_INTERVAL: Duration = Duration::from_secs(60);

impl DeveloperPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let queries = DevQueries::global(cx);
        let authenticated = state::user(cx).read(cx).is_authenticated();
        queries.update(cx, |q, cx| {
            q.begin_session();
            if authenticated {
                q.fetch_owned(cx);
            }
        });
        let weak = cx.entity().downgrade();
        let toolbar = {
            let queries = queries.clone();
            cx.new(|cx| DeveloperToolbar::new(weak, queries, cx))
        };
        cx.observe(&queries, |this, _, cx| {
            this.sync_slot(cx);
            cx.notify();
        })
        .detach();
        cx.observe(&state::user(cx), |this, _, cx| {
            this.sync_slot(cx);
            cx.notify();
        })
        .detach();
        // Refetch the list every minute while it is shown.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;
                let alive = this.update(cx, |this, cx| {
                    if this.view == View::List && state::user(cx).read(cx).is_authenticated() {
                        this.queries.update(cx, |q, cx| q.fetch_owned(cx));
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
        let mut this = Self {
            view: View::List,
            selection: None,
            scope: DeveloperScope::All,
            queries,
            toolbar,
            publish_form: None,
            detail: None,
            slot_shown: false,
        };
        this.sync_slot(cx);
        this
    }

    fn host(&self, cx: &Context<Self>) -> Host {
        Host { page: cx.entity().downgrade(), queries: self.queries.clone() }
    }

    /// The toolbar lives in the title bar only while the list is shown and the data is loaded.
    fn sync_slot(&mut self, cx: &mut Context<Self>) {
        let want = self.view == View::List
            && state::user(cx).read(cx).is_authenticated()
            && self.queries.read(cx).owned(cx).data.is_some();
        if want && !self.slot_shown {
            titlebar::set_slot(cx, self.toolbar.clone());
            self.slot_shown = true;
        } else if !want && self.slot_shown {
            titlebar::clear_slot(cx);
            self.slot_shown = false;
        }
    }

    fn set_selection(&mut self, selection: Option<Selection>, cx: &mut Context<Self>) {
        self.selection = selection;
        cx.notify();
    }

    pub(crate) fn change_scope(&mut self, scope: DeveloperScope, cx: &mut Context<Self>) {
        self.scope = scope;
        self.selection = None;
        cx.notify();
    }

    pub(crate) fn open_form(&mut self, cx: &mut Context<Self>) {
        // Needs a window to build the form fields: defer to the next render.
        self.view = View::Form;
        self.sync_slot(cx);
        cx.notify();
    }

    fn ensure_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.publish_form.is_some() {
            return;
        }
        let form = cx.new(|cx| PublishAddonForm::new(window, cx));
        cx.subscribe(&form, |this, _, _: &PublishFormEvent, cx| this.close_form(cx)).detach();
        self.publish_form = Some(form);
    }

    fn close_form(&mut self, cx: &mut Context<Self>) {
        self.view = View::List;
        self.publish_form = None;
        // Back on the list: fetch the owned addons again so it is current.
        self.queries.update(cx, |q, cx| q.fetch_owned(cx));
        self.sync_slot(cx);
        cx.notify();
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        DevQueries::retry(&self.queries, cx).detach();
    }
}

impl Render for DeveloperPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        if !state::user(cx).read(cx).is_authenticated() {
            return div().into_any_element();
        }
        if self.view == View::Form {
            self.ensure_form(window, cx);
            return div().size_full().min_h_0().child(self.publish_form.clone().unwrap()).into_any_element();
        }
        let owned = self.queries.read(cx).owned(cx);
        let root = div().flex().flex_col().size_full().min_h_0();

        let Some(data) = owned.data else {
            return match owned.error {
                None => {
                    root.child(loading_block("Loading your addons...", cx).flex_1().py(px(64.0))).into_any_element()
                }
                Some(error) => root
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .py(px(64.0))
                            .text_color(t.muted_foreground)
                            .child(Icon::new(icons::TRIANGLE_ALERT).size(px(32.0)).text_color(t.destructive))
                            .child(div().mt(px(12.0)).text_sm_tw().child(error))
                            .child(
                                Button::new("dev-load-retry")
                                    .mt(px(16.0))
                                    .label("Retry")
                                    .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                            ),
                    )
                    .into_any_element(),
            };
        };

        if data.addons.is_empty() && data.submissions.is_empty() {
            return root
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p(px(32.0))
                        .child(
                            div()
                                .mb(px(16.0))
                                .rounded_full()
                                .bg(alpha(t.primary, 0.1))
                                .p(px(16.0))
                                .child(Icon::new(icons::CODE_XML).size(px(32.0)).text_color(t.primary)),
                        )
                        .child(div().mb(px(8.0)).text_xl_tw().font_weight(FontWeight::SEMIBOLD).child("No addons yet"))
                        .child(
                            div()
                                .mb(px(24.0))
                                .max_w(px(384.0))
                                .text_center()
                                .text_sm_tw()
                                .text_color(t.muted_foreground)
                                .child("Publish an addon to list it here."),
                        )
                        .child(
                            Button::new("dev-empty-new")
                                .icon(icons::PLUS)
                                .label("New addon")
                                .on_click(cx.listener(|this, _, _, cx| this.open_form(cx))),
                        ),
                )
                .into_any_element();
        }

        root.child(self.workspace(&data, window, cx)).into_any_element()
    }
}

impl DeveloperPage {
    /// Grouped list on the left, selected detail on the right.
    fn workspace(
        &mut self,
        data: &OwnedAddonsData,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let t = cx.tokens();
        let published: HashSet<&str> = data.addons.iter().map(|a| a.name.as_str()).collect();
        let visible_submissions: Vec<&OwnedSubmission> =
            data.submissions.iter().filter(|s| s.is_standalone(&published)).collect();
        let fallback = match self.scope {
            DeveloperScope::Addons => data.addons.first().map(|a| Selection::Addon(a.name.clone())),
            DeveloperScope::Submissions => visible_submissions.first().map(|s| Selection::Submission(s.id)),
            DeveloperScope::All => visible_submissions
                .first()
                .map(|s| Selection::Submission(s.id))
                .or_else(|| data.addons.first().map(|a| Selection::Addon(a.name.clone()))),
        };
        let addon_of = |selection: &Selection| match selection {
            Selection::Addon(name) => data.addons.iter().find(|a| &a.name == name),
            Selection::Submission(_) => None,
        };
        let submission_of = |selection: &Selection| match selection {
            Selection::Submission(id) => data.submissions.iter().find(|s| s.id == *id),
            Selection::Addon(_) => None,
        };
        // Polling may remove a submission after publication, fall back without retaining an old form.
        let selected =
            self.selection.clone().filter(|s| addon_of(s).is_some() || submission_of(s).is_some()).or(fallback);
        let Some(selected) = selected else { return div().into_any_element() };

        let host = self.host(cx);
        let selected_addon = addon_of(&selected);
        let selected_submission = submission_of(&selected);

        // Build a new detail entity when the selection changes, otherwise hand the fresh data to the existing one.
        let detail_matches = self.detail.as_ref().is_some_and(|d| d.shows(&selected));
        if !detail_matches {
            self.detail = None;
            if let Some(addon) = selected_addon {
                let (addon, url, host) = (addon.clone(), data.webhook_install_url.clone(), host.clone());
                let name = addon.name.clone();
                let view = cx.new(|cx| AddonDetails::new(addon, url, host, cx));
                self.detail = Some(Detail::Addon(name, view));
            } else if let Some(submission) = selected_submission {
                let back_to = back_to(data, submission);
                let (submission, host) = (submission.clone(), host.clone());
                let id = submission.id;
                let view = cx.new(|cx| SubmissionDetails::new(submission, back_to, host, cx));
                self.detail = Some(Detail::Submission(id, view));
            }
        } else if let Some(detail) = &self.detail {
            match (detail, selected_addon, selected_submission) {
                (Detail::Addon(_, view), Some(addon), _) => {
                    let (addon, url) = (addon.clone(), data.webhook_install_url.clone());
                    view.update(cx, |v, _| v.set_props(addon, url));
                }
                (Detail::Submission(_, view), _, Some(submission)) => {
                    let back = back_to(data, submission);
                    let submission = submission.clone();
                    view.update(cx, |v, _| v.set_props(submission, back));
                }
                _ => {}
            }
        }

        let show_submissions = self.scope != DeveloperScope::Addons && !visible_submissions.is_empty();
        let show_published = self.scope != DeveloperScope::Submissions && !data.addons.is_empty();
        let mut groups = div().flex().flex_col().gap(px(24.0)).p(px(12.0));
        if show_submissions {
            groups = groups.child(group_section(
                "Submissions",
                visible_submissions.len(),
                visible_submissions.iter().map(|s| list_row_submission(s, &selected, cx)).collect(),
                cx,
            ));
        }
        if show_published {
            groups = groups.child(group_section(
                "Published",
                data.addons.len(),
                data.addons.iter().map(|a| list_row_addon(a, &selected, cx)).collect(),
                cx,
            ));
        }

        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex_none()
                    .w(px(220.0))
                    .h_full()
                    .min_h_0()
                    .border_r_1()
                    .border_color(t.border)
                    .child(ScrollArea::new("dev-list-scroll").size_full().child(groups)),
            )
            .child(div().flex_1().min_w_0().min_h_0().h_full().children(self.detail.as_ref().map(|d| d.view())))
            .into_any_element()
    }
}

/// A submission reached from an addon's review history returns to that addon.
fn back_to(data: &OwnedAddonsData, submission: &OwnedSubmission) -> Option<(Selection, String)> {
    data.addons.iter().find(|a| a.name == submission.name).map(|a| (Selection::Addon(a.name.clone()), a.alias.clone()))
}

fn group_section(
    title: &'static str,
    count: usize,
    rows: Vec<gpui_kit::AnyElement>,
    cx: &Context<DeveloperPage>,
) -> impl IntoElement {
    let t = cx.tokens();
    div()
        .child(
            div()
                .mb(px(8.0))
                .flex()
                .justify_between()
                .px(px(8.0))
                .text_xs_tw()
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.muted_foreground)
                .child(title)
                .child(count.to_string()),
        )
        .child(div().flex().flex_col().gap(px(4.0)).children(rows))
}

fn list_row(
    selection: Selection,
    selected: &Selection,
    leading: gpui_kit::AnyElement,
    title: String,
    subtitle: String,
    subtitle_color: gpui_kit::Hsla,
    cx: &Context<DeveloperPage>,
) -> gpui_kit::AnyElement {
    let t = cx.tokens();
    let is_selected = &selection == selected;
    div()
        .id(selection.element_id())
        .flex()
        .w_full()
        .items_center()
        .gap(px(12.0))
        .rounded(radius::LG)
        .border_1()
        .border_color(if is_selected { alpha(t.primary, 0.2) } else { gpui_kit::transparent_black() })
        .when(is_selected, |d| d.bg(alpha(t.primary, 0.1)))
        .px(px(8.0))
        .py(px(12.0))
        .cursor_pointer()
        .when(!is_selected, |d| d.hover(|s| s.bg(alpha(t.muted, 0.5))))
        .child(leading)
        .child(
            div()
                .min_w_0()
                .child(div().truncate().text_sm_tw().font_weight(FontWeight::MEDIUM).child(title))
                .child(div().mt(px(4.0)).text_xs_tw().text_color(subtitle_color).child(subtitle)),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.set_selection(Some(selection.clone()), cx)))
        .into_any_element()
}

fn list_row_addon(
    addon: &owned::OwnedAddon,
    selected: &Selection,
    cx: &Context<DeveloperPage>,
) -> gpui_kit::AnyElement {
    let t = cx.tokens();
    let icon_url = addon_icon_url(addon.icon.as_deref(), Some(&addon.repo), addon.branch.as_deref(), true);
    list_row(
        Selection::Addon(addon.name.clone()),
        selected,
        AddonIcon::new(addon.alias.clone(), icon_url).into_any_element(),
        addon.alias.clone(),
        format!("{} downloads", fmt::number(addon.downloads)),
        t.muted_foreground,
        cx,
    )
}

fn list_row_submission(
    submission: &OwnedSubmission,
    selected: &Selection,
    cx: &Context<DeveloperPage>,
) -> gpui_kit::AnyElement {
    let t = cx.tokens();
    let (text, color) = match submission.status {
        owned::OwnedSubmissionStatus::Open => ("In review", t.primary),
        owned::OwnedSubmissionStatus::Rejected => ("Changes requested", t.destructive),
    };
    list_row(
        Selection::Submission(submission.id),
        selected,
        Icon::new(icons::INBOX).size(px(20.0)).text_color(t.muted_foreground).flex_none().into_any_element(),
        submission.name.clone(),
        text.to_string(),
        color,
        cx,
    )
}
