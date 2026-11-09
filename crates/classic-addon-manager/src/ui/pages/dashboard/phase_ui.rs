//! Parts shared by the update and version dialogs: the phase of the update they run through the addon store, the
//! progress steps and bar, the error alert, keyboard key chips and small uppercase captions.
use std::time::Duration;

use cam_ui::{
    components::{error_box, spinner},
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, SharedString,
    Styled, div, linear_color_stop, linear_gradient, px, relative,
};

use crate::state::{AddonState, Operation};

/// Why an update or install failed, as the error alert shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub title: String,
    pub description: String,
}

/// The update a dialog runs through the addon store.
pub enum Phase<T> {
    Idle,
    /// `started` is what the dialog shows while the update runs. `op` is the store's operation as last seen. It stays
    /// until the dialog has the result, also when the store ended the operation a moment before.
    Running {
        started: T,
        op: Operation,
    },
    Failed(Failure),
}

impl<T> Phase<T> {
    pub fn running(started: T) -> Self {
        Phase::Running { started, op: Operation::default() }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Phase::Running { .. })
    }

    /// While running, takes the store's latest operation on `name`.
    pub fn follow(&mut self, store: &AddonState, name: &str) {
        if let (Phase::Running { op, .. }, Some(latest)) = (self, store.operation(name)) {
            *op = latest.clone();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Done,
    Active,
    Pending,
}

/// Small chip showing a keyboard key. `strong` is the variant used inside the primary button.
pub fn kbd(text: &'static str, strong: bool, cx: &App) -> gpui_kit::Div {
    let t = cx.tokens();
    let base = div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(20.0))
        .min_w(px(20.0))
        .px_1()
        .rounded(radius::MD)
        .border_1()
        .font_mono_tw()
        .text_size(px(10.0))
        .line_height(px(10.0))
        .child(text);
    if strong {
        base.border_color(alpha(t.primary_foreground, 0.2))
            .bg(alpha(t.primary_foreground, 0.1))
            .text_color(alpha(t.primary_foreground, 0.8))
    } else {
        base.border_color(t.border).bg(t.card).text_color(t.muted_foreground)
    }
}

/// Small uppercase caption above a value, such as "Installed" over a version. Callers pick its colour.
pub fn eyebrow(text: &'static str) -> gpui_kit::Div {
    div().text_size(px(10.0)).line_height(px(16.0)).font_weight(FontWeight::SEMIBOLD).child(text.to_uppercase())
}

/// Text on the right of a step row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepNote {
    /// `done` or `…` depending on the row state, or nothing.
    Status,
    /// `236 KB / 482 KB` while downloading, `12 / 40 files` while extracting.
    Progress(String),
    /// Size of an archive that matched its published SHA-256, like `482 KB`.
    Verified(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRow {
    pub label: String,
    pub state: StepState,
    pub note: StepNote,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProgressBar {
    /// Unknown duration: a shimmer over an empty track.
    Shimmer(String),
    /// Download or extraction running: `fraction` of the track is filled.
    Fill { fraction: f32, label: String },
}

/// Rows and bar shown while an install runs.
#[derive(Clone, Debug, PartialEq)]
pub struct RunningView {
    pub rows: [StepRow; 3],
    pub bar: ProgressBar,
}

/// Formats a size like `482 KB` or `1.4 MB`.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{:.1} MB", b / (KB * KB))
    }
}

/// Steps are: resolve manifest, download, install. Missing dependencies are downloaded and extracted during the
/// download step, before `parent` at `tag`. The bar shows the current stage of `op`: bytes, then files.
pub fn running_view(parent: &str, tag: &str, op: &Operation, running_label: &str) -> RunningView {
    use StepState::{Active, Done, Pending};
    let manifest_resolved = op.manifest_resolved;
    let row = |label: String, state, note| StepRow { label, state, note };
    let manifest =
        row("Resolving manifest".to_string(), if manifest_resolved { Done } else { Active }, StepNote::Status);
    let downloading = format!("Downloading {tag}");
    let installing = || row("Installing".to_string(), Pending, StepNote::Status);

    let Some(p) = op.progress.as_ref().filter(|_| manifest_resolved) else {
        return RunningView {
            rows: [
                manifest,
                row(downloading, if manifest_resolved { Active } else { Pending }, StepNote::Status),
                installing(),
            ],
            bar: ProgressBar::Shimmer(running_label.to_string()),
        };
    };
    let bytes = || StepNote::Progress(format!("{} / {}", format_bytes(p.received), format_bytes(p.total)));
    let files = || {
        if p.files_total == 0 {
            StepNote::Status
        } else {
            StepNote::Progress(format!("{} / {} files", p.files_extracted, p.files_total))
        }
    };
    let rows = match (p.name == parent, p.verified) {
        (true, false) => [manifest, row(downloading, Active, bytes()), installing()],
        (true, true) => [
            manifest,
            row(downloading, Done, StepNote::Verified(format_bytes(p.total))),
            row("Installing".to_string(), Active, files()),
        ],
        (false, false) => {
            [manifest, row(format!("Downloading {} (dependency)", p.alias), Active, bytes()), installing()]
        }
        (false, true) => [manifest, row(format!("Installing {} (dependency)", p.alias), Active, files()), installing()],
    };
    let bar = match p.fraction() {
        Some(fraction) => ProgressBar::Fill {
            fraction: fraction as f32,
            label: format!(
                "{}… {}%",
                if p.verified { "Extracting" } else { "Downloading" },
                (fraction * 100.0).floor() as u32
            ),
        },
        None => ProgressBar::Shimmer(running_label.to_string()),
    };
    RunningView { rows, bar }
}

/// Step rows and progress bar shown while an install or update runs.
pub fn running_steps(id: &'static str, view: RunningView, cx: &App) -> gpui_kit::Div {
    let t = cx.tokens();
    let green = tw(Tw::Green, 500);
    let rows = view.rows.into_iter().map(|StepRow { label, state, note }| {
        let marker = match state {
            StepState::Done => div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(20.0))
                .rounded_full()
                .bg(alpha(green, 0.15))
                .child(Icon::new(icons::CHECK).size(px(12.0)).text_color(green))
                .into_any_element(),
            StepState::Active => spinner().with_size(px(20.0)).color(t.primary).into_any_element(),
            StepState::Pending => {
                div().size(px(20.0)).rounded_full().border_2().border_color(t.border).into_any_element()
            }
        };
        let note: AnyElement = match note {
            StepNote::Status => div()
                .text_color(t.muted_foreground)
                .child(match state {
                    StepState::Done => "done",
                    StepState::Active => "…",
                    StepState::Pending => "",
                })
                .into_any_element(),
            StepNote::Progress(text) => {
                div().font_mono_tw().text_color(t.muted_foreground).child(SharedString::from(text)).into_any_element()
            }
            StepNote::Verified(size) => div()
                .flex()
                .items_center()
                .gap_1()
                .font_mono_tw()
                .text_color(green)
                .child(SharedString::from(format!("{size} · SHA-256")))
                .child(Icon::new(icons::CHECK).size(px(12.0)))
                .into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap_3()
            .text_sm_tw()
            .child(marker)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(if state == StepState::Pending { t.muted_foreground } else { t.foreground })
                    .when_active(state == StepState::Active)
                    .child(SharedString::from(label)),
            )
            .child(div().ml_auto().flex_none().text_xs_tw().child(note))
    });

    // gpui clips children to a plain rectangle, not to rounded corners. So each layer rounds its own corners (track
    // radius minus the 1px border) and the shimmer stays inside the track.
    let inner = radius::XL - px(1.0);
    let half = |from: gpui_kit::Hsla, to: gpui_kit::Hsla| {
        div().h_full().w(relative(0.5)).bg(linear_gradient(
            90.0,
            linear_color_stop(from, 0.0),
            linear_color_stop(to, 1.0),
        ))
    };
    let clear = gpui_kit::transparent_black();
    let mid = alpha(t.primary, 0.35);
    let shimmer = || {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(relative(0.5))
            .flex()
            .child(half(clear, mid).rounded_l(inner))
            .child(half(mid, clear).rounded_r(inner))
            .with_animation(
                (ElementId::from(id), "shimmer"),
                Animation::new(Duration::from_millis(2800)).repeat(),
                // Moves right and back inside the track. Its left edge goes between 0% and 50%.
                |el, v| el.left(relative(0.25 * (1.0 - (v * std::f32::consts::TAU).cos()))),
            )
    };
    let (layer, label): (AnyElement, String) = match view.bar {
        ProgressBar::Shimmer(label) => (shimmer().into_any_element(), label),
        ProgressBar::Fill { fraction, label } => (
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .w(relative(fraction))
                .rounded(inner)
                .bg(alpha(t.primary, 0.35))
                .into_any_element(),
            label,
        ),
    };

    div().flex().flex_col().gap_3().children(rows).child(
        div()
            .relative()
            .h(px(40.0))
            .overflow_hidden()
            .rounded(radius::XL)
            .bg(alpha(t.primary, 0.1))
            .border_1()
            .border_color(alpha(t.primary, 0.2))
            .child(layer)
            .child(
                div()
                    .relative()
                    .flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .text_sm_tw()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.primary)
                    .child(SharedString::from(label)),
            ),
    )
}

trait WhenActive: Styled + Sized {
    fn when_active(self, active: bool) -> Self {
        if active { self.font_weight(FontWeight::MEDIUM) } else { self }
    }
}
impl<T: Styled + Sized> WhenActive for T {}

/// Alert box shown after a failed install or update.
pub fn error_alert(failure: &Failure, cx: &App) -> gpui_kit::Div {
    error_box(&cx.tokens(), failure.title.clone(), failure.description.clone()).flex_none().mx_6().mt_4()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cam_core::addon::InstallProgress;

    fn progress(name: &str, received: u64, verified: bool) -> InstallProgress {
        InstallProgress {
            name: name.into(),
            alias: format!("{name}-alias"),
            tag: "v2".into(),
            received,
            total: 482_113,
            verified,
            ..Default::default()
        }
    }

    fn extracting(name: &str, files_extracted: u64, files_total: u64) -> InstallProgress {
        InstallProgress { files_extracted, files_total, ..progress(name, 482_113, true) }
    }

    fn op(manifest_resolved: bool, progress: InstallProgress) -> Operation {
        Operation { manifest_resolved, progress: Some(progress) }
    }

    fn states(v: &RunningView) -> [StepState; 3] {
        v.rows.each_ref().map(|r| r.state)
    }

    fn fill(v: &RunningView) -> (f32, &str) {
        let ProgressBar::Fill { fraction, label } = &v.bar else { panic!("expected a filling bar, got {:?}", v.bar) };
        (*fraction, label)
    }

    #[test]
    fn running_view_follows_download_then_extraction() {
        use StepState::*;
        let v = running_view("main", "v2", &op(false, progress("main", 10, false)), "Updating…");
        assert_eq!(states(&v), [Active, Pending, Pending], "progress is ignored before the manifest resolved");

        let v = running_view("main", "v2", &op(true, progress("main", 241_057, false)), "Updating…");
        assert_eq!(states(&v), [Done, Active, Pending]);
        assert_eq!(v.rows[1].note, StepNote::Progress("235 KB / 471 KB".into()));
        let (fraction, label) = fill(&v);
        assert!((fraction - 0.5).abs() < 1e-4);
        assert_eq!(label, "Downloading… 50%");

        // verified, extraction not started yet
        let v = running_view("main", "v2", &op(true, progress("main", 482_113, true)), "Updating…");
        assert_eq!(states(&v), [Done, Done, Active]);
        assert_eq!(v.rows[1].note, StepNote::Verified("471 KB".into()));
        assert_eq!(v.bar, ProgressBar::Shimmer("Updating…".into()));

        // the bar restarts for the files
        let v = running_view("main", "v2", &op(true, extracting("main", 12, 40)), "Updating…");
        assert_eq!(states(&v), [Done, Done, Active]);
        assert_eq!(v.rows[2].note, StepNote::Progress("12 / 40 files".into()));
        assert_eq!(fill(&v), (0.3, "Extracting… 30%"));
    }

    #[test]
    fn dependencies_stay_in_the_download_step() {
        let v = running_view("main", "v2", &op(true, progress("lib", 0, false)), "Updating…");
        assert_eq!(v.rows[1].label, "Downloading lib-alias (dependency)");
        let v = running_view("main", "v2", &op(true, extracting("lib", 5, 10)), "Updating…");
        assert_eq!(
            (v.rows[1].label.as_str(), v.rows[1].state),
            ("Installing lib-alias (dependency)", StepState::Active)
        );
        assert_eq!(v.rows[1].note, StepNote::Progress("5 / 10 files".into()));
        assert_eq!(v.rows[2].state, StepState::Pending);
        assert_eq!(fill(&v), (0.5, "Extracting… 50%"));
    }

    #[test]
    fn byte_sizes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(482_113), "471 KB");
        assert_eq!(format_bytes(1_468_006), "1.4 MB");
    }
}
