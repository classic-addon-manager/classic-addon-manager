//! Title-bar slot with the scope toggle (All / Addons / Submissions), Refresh and New addon.
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use cam_ui::{
    components::{Button, ButtonSize, ButtonVariant, ToggleGroup, ToggleGroupItem, ToggleSize, ToggleVariant, spinner},
    icons,
    theme::{TokensExt, alpha},
};
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Task, WeakEntity, Window, div, px,
};

use super::{DeveloperPage, queries::DevQueries};
use crate::state;
use crate::ui::shell::titlebar::NoDrag as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeveloperScope {
    All,
    Addons,
    Submissions,
}

impl DeveloperScope {
    fn value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Addons => "addons",
            Self::Submissions => "submissions",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "addons" => Some(Self::Addons),
            "submissions" => Some(Self::Submissions),
            _ => None,
        }
    }
}

const HIGHLIGHT_TIMEOUT: Duration = Duration::from_secs(60);

pub struct DeveloperToolbar {
    page: WeakEntity<DeveloperPage>,
    queries: Entity<DevQueries>,
    show_spinner: bool,
    highlight_submissions: bool,
    previous: Option<HashSet<String>>,
    highlight_task: Option<Task<()>>,
}

impl DeveloperToolbar {
    pub fn new(page: WeakEntity<DeveloperPage>, queries: Entity<DevQueries>, cx: &mut Context<Self>) -> Self {
        cx.observe(&queries, |this, _, cx| {
            this.observe_data(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            page,
            queries,
            show_spinner: false,
            highlight_submissions: false,
            previous: None,
            highlight_task: None,
        };
        this.observe_data(cx);
        this
    }

    /// Highlight the Submissions segment when a poll adds a submission or changes its status.
    fn observe_data(&mut self, cx: &mut Context<Self>) {
        let Some(data) = self.queries.read(cx).owned(cx).data else { return };
        let signatures: HashSet<String> = data.submissions.iter().map(|s| format!("{}:{:?}", s.id, s.status)).collect();
        let previous = self.previous.replace(signatures.clone());
        let Some(previous) = previous else { return };
        if signatures.iter().all(|s| previous.contains(s)) {
            return;
        }
        self.highlight_submissions = true;
        self.highlight_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HIGHLIGHT_TIMEOUT).await;
            let _ = this.update(cx, |this, cx| {
                this.highlight_submissions = false;
                this.highlight_task = None;
                cx.notify();
            });
        }));
    }

    fn handle_refresh(&mut self, cx: &mut Context<Self>) {
        if self.show_spinner {
            return;
        }
        self.show_spinner = true;
        cx.notify();
        let started = Instant::now();
        let retry = DevQueries::retry(&self.queries, cx);
        cx.spawn(async move |this, cx| {
            retry.await;
            state::hold_spinner(started, cx).await;
            let _ = this.update(cx, |this, cx| {
                this.show_spinner = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn handle_scope_change(&mut self, value: &SharedString, cx: &mut Context<Self>) {
        let Some(scope) = DeveloperScope::parse(value) else { return };
        self.highlight_task = None;
        self.highlight_submissions = false;
        let _ = self.page.update(cx, |page, cx| page.change_scope(scope, cx));
        cx.notify();
    }
}

impl Render for DeveloperToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let owned = self.queries.read(cx).owned(cx);
        let Some(data) = owned.data else { return div().into_any_element() };
        let scope = self.page.upgrade().map(|p| p.read(cx).scope).unwrap_or(DeveloperScope::All);
        let published: HashSet<&str> = data.addons.iter().map(|a| a.name.as_str()).collect();
        let visible_submissions = data.submissions.iter().filter(|s| s.is_standalone(&published)).count();
        let failed = owned.last_attempt_failed;
        let busy = self.show_spinner;
        let icon_color = t.foreground;

        let mut submissions_item =
            ToggleGroupItem::new("submissions").label(format!("Submissions ({visible_submissions})"));
        if self.highlight_submissions {
            submissions_item = submissions_item.bg(alpha(t.primary, 0.2)).text_color(t.primary);
        }

        let mut refresh = Button::new("dev-refresh")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .h(px(32.0))
            .has_icon(true)
            .disabled(busy)
            .child(div().mr(px(6.0)).flex().items_center().child(if busy {
                spinner().with_size(px(14.0)).into_any_element()
            } else {
                Icon::new(icons::REFRESH_CW).size(px(14.0)).text_color(icon_color).into_any_element()
            }))
            .child("Refresh")
            .on_click(cx.listener(|this, _, _, cx| this.handle_refresh(cx)));
        if failed {
            refresh =
                refresh.text_color(t.destructive).border_color(alpha(t.destructive, 0.4)).bg(alpha(t.destructive, 0.1));
        }

        div()
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(
                div().no_drag().flex().flex_none().items_center().gap(px(8.0)).child(
                    ToggleGroup::new("dev-scope")
                        .variant(ToggleVariant::Outline)
                        .size(ToggleSize::Sm)
                        .value(scope.value())
                        .on_value_change(cx.listener(|this, v: &SharedString, _, cx| this.handle_scope_change(v, cx)))
                        .item(ToggleGroupItem::new("all").label("All"))
                        .item(ToggleGroupItem::new("addons").label(format!("Addons ({})", data.addons.len())))
                        .item(submissions_item),
                ),
            )
            .child(
                div().no_drag().flex().flex_none().items_center().gap(px(8.0)).child(refresh).child(
                    Button::new("dev-new-addon")
                        .size(ButtonSize::Sm)
                        .h(px(32.0))
                        .icon(icons::PLUS)
                        .label("New addon")
                        .on_click(cx.listener(|this, _, _, cx| {
                            let _ = this.page.update(cx, |page, cx| page.open_form(cx));
                        })),
                ),
            )
            .into_any_element()
    }
}
