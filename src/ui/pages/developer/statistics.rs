//! Statistics pane of an addon: metrics, download and subscriber charts, ratings and version adoption. The charts are
//! gpui-component's `BarChart` and `AreaChart`, which lay out the axes, grid and hover tooltip themselves. This file
//! picks the days a chart shows and formats the figures.
use chrono::{Duration, NaiveDate, Utc};
use gpui_kit::component::{
    Icon, Sizable as _,
    chart::{AreaChart, BarChart, LineChart},
    plot::{
        scale::{Scale, ScaleBand},
        shape::BarAlignment,
    },
};
use gpui_kit::{
    Context, Corners, Div, Entity, FontWeight, Hsla, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, prelude::FluentBuilder as _, px, relative,
};

use super::{
    queries::DevQueries,
    types::{AddonRatingStats, AddonVersionStats, SnapshotPoint, VersionLagBucket},
};
use crate::ui::{
    components::{Button, ButtonSize, ButtonVariant, spinner},
    icons,
    theme::{Tokens, TokensExt, TwText, alpha},
    utils::format_number,
};

// ---------------------------------------------------------------------------------------------------------------------
// Series window and formatting
// ---------------------------------------------------------------------------------------------------------------------
/// One day of a chart series. A figure is `None` when it is not available for that day.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartPoint {
    pub taken_on: NaiveDate,
    pub daily_downloads: Option<i64>,
    pub subscribers: Option<i64>,
    pub daily_subscriber_change: Option<i64>,
}

/// The last `days` UTC days ending today. Missing days become all-`None` points (kept on the axis without inventing
/// counts), and the result is empty when no point of the window has a subscriber count.
pub fn series_in_window(series: &[SnapshotPoint], days: u32, today: NaiveDate) -> Vec<ChartPoint> {
    let start = today + Duration::days(1 - i64::from(days));
    let points: Vec<ChartPoint> = (0..i64::from(days))
        .map(|i| {
            let taken_on = start + Duration::days(i);
            match series.iter().find(|p| p.taken_on == taken_on) {
                Some(p) => ChartPoint {
                    taken_on,
                    daily_downloads: p.daily_downloads,
                    subscribers: Some(p.subscribers),
                    daily_subscriber_change: p.daily_subscriber_change,
                },
                None => {
                    ChartPoint { taken_on, daily_downloads: None, subscribers: None, daily_subscriber_change: None }
                }
            }
        })
        .collect();
    if points.iter().any(|p| p.subscribers.is_some()) { points } else { Vec::new() }
}

/// Short day such as `Oct 5`.
pub fn format_snapshot_day(day: NaiveDate) -> String {
    day.format("%b %-d").to_string()
}

/// Ratio as a percentage with at most one decimal, such as `12.5%`.
pub fn format_ratio(value: f64) -> String {
    let tenths = (value * 1000.0 * (1.0 + 1e-12)).round() as i64;
    let (int, frac) = (tenths.abs() / 10, tenths.abs() % 10);
    let sign = if tenths < 0 { "-" } else { "" };
    if frac == 0 { format!("{sign}{}%", format_number(int)) } else { format!("{sign}{}.{frac}%", format_number(int)) }
}

// ---------------------------------------------------------------------------------------------------------------------
// Charts
// ---------------------------------------------------------------------------------------------------------------------

/// Most x labels a chart shows, spread evenly from the first day to the last. A 7-day chart labels every day.
const X_LABELS: usize = 7;

/// One day of a daily chart: its x label and its figure, `None` when none was recorded.
type Day = (SharedString, Option<i64>);

fn days(points: &[ChartPoint], figure: impl Fn(&ChartPoint) -> Option<i64>) -> Vec<Day> {
    points.iter().map(|p| (format_snapshot_day(p.taken_on).into(), figure(p))).collect()
}

/// Gap between bars as a share of their band. A bar chart picks the hovered band by rounding from the start of a band,
/// so the hover moves to the next band halfway between two band starts. At half a band or more, every bar ends before
/// that point and the tooltip names the bar under the pointer.
const BAR_GAP: f32 = 0.5;

/// Bar chart of a daily figure with one band per day. A day without a figure keeps its band, draws no bar, and its
/// tooltip says `No data`. `id` tells the bar charts of the pane apart, since they are built at the same place.
fn daily_bars(
    id: &'static str,
    days: Vec<Day>,
    name: &'static str,
    color: impl Fn(Option<i64>) -> Hsla + 'static,
) -> BarChart<Day, SharedString, f64> {
    BarChart::new(days)
        .id(id)
        .band(|d: &Day| d.0.clone())
        .value(|d: &Day| d.1.unwrap_or(0) as f64)
        .fill(move |d: &Day, _, _, _| color(d.1))
        .corner_radii(Corners::all(px(2.0)))
        .padding_inner(BAR_GAP)
        .band_tick_count(X_LABELS)
        .name(name)
        .tooltip_value(|d: &Day, _| d.1.map_or_else(|| "No data".into(), |v| format_number(v).into()))
}

/// One bar of the version chart.
struct VersionBar {
    release_id: i64,
    tag: SharedString,
    subscribers: i64,
    share: f64,
    bucket: VersionLagBucket,
    color: Hsla,
}

/// Band of a version bar: the release id, which tells apart two releases with the same tag, and the tag.
#[derive(Clone, PartialEq, Eq, Hash)]
struct VersionBand(i64, SharedString);

impl From<VersionBand> for SharedString {
    fn from(band: VersionBand) -> Self {
        band.1
    }
}

/// Thickest a version bar gets, and the space before the first bar and after the last, as a share of a band. The
/// version chart and its tag column both lay their bands out with these, so each tag sits level with its bar.
const VERSION_BAR_MAX: f32 = 30.0;
const VERSION_OUTER_GAP: f32 = 0.2;
/// Line height of a tag in the tag column.
const VERSION_TAG_HEIGHT: f32 = 16.0;

/// The tags of the version chart, right-aligned in a column `height` tall, each centred on its bar. The chart's own
/// labels sit 2 px from the bars, a gap it has no setting for, so the chart draws none and this column takes their
/// place.
fn version_tags(bars: &[VersionBar], height: f32, t: &Tokens) -> Div {
    let scale = ScaleBand::new(0..bars.len(), [0.0, height])
        .padding_inner(BAR_GAP)
        .padding_outer(VERSION_OUTER_GAP)
        .max_band_width(VERSION_BAR_MAX);
    let half_bar = scale.band_width() / 2.0;
    let mut top = 0.0;
    div().flex_none().flex().flex_col().items_end().text_xs_tw().text_color(t.muted_foreground).children(
        bars.iter().enumerate().map(|(i, bar)| {
            let tag_top = scale.tick(&i).unwrap_or(0.0) + half_bar - VERSION_TAG_HEIGHT / 2.0;
            let gap = tag_top - top;
            top = tag_top + VERSION_TAG_HEIGHT;
            div().mt(px(gap)).h(px(VERSION_TAG_HEIGHT)).line_height(px(VERSION_TAG_HEIGHT)).child(bar.tag.clone())
        }),
    )
}

const LAG_ORDER: [VersionLagBucket; 4] = VersionLagBucket::ALL;

fn lag_label(b: VersionLagBucket) -> &'static str {
    match b {
        VersionLagBucket::Latest => "Latest",
        VersionLagBucket::OneBehind => "1 behind",
        VersionLagBucket::TwoBehind => "2+ behind",
        VersionLagBucket::Unavailable => "Unavailable",
    }
}

fn lag_color(b: VersionLagBucket, t: &Tokens) -> Hsla {
    match b {
        VersionLagBucket::Latest => t.primary,
        VersionLagBucket::OneBehind => alpha(t.primary, 0.6),
        VersionLagBucket::TwoBehind => alpha(t.primary, 0.35),
        VersionLagBucket::Unavailable => alpha(t.destructive, 0.7),
    }
}

fn chart_box(height: f32, chart: impl IntoElement) -> Div {
    div().w_full().h(px(height)).flex_none().child(chart)
}

fn note(text: &'static str, t: &Tokens) -> Div {
    div().text_xs_tw().text_color(t.muted_foreground).child(text)
}

fn empty(text: &'static str, t: &Tokens) -> Div {
    div().text_sm_tw().text_color(t.muted_foreground).child(text)
}

fn metric(label: &'static str, value: String, t: &Tokens) -> Div {
    div()
        .child(div().text_xs_tw().text_color(t.muted_foreground).child(label))
        .child(div().mt_1().text_2xl_tw().child(value))
}

fn heading(text: &'static str) -> Div {
    div().flex().items_center().gap_2().text_sm_tw().font_weight(FontWeight::MEDIUM).child(text)
}

fn daily_download_chart(points: &[ChartPoint], t: &Tokens) -> Div {
    if points.is_empty() {
        return empty("No snapshots in this period.", t);
    }
    if points.iter().all(|p| p.daily_downloads.is_none()) {
        return empty("No daily download deltas in this period.", t);
    }
    let primary = t.primary;
    let chart = daily_bars("stats-downloads", days(points, |p| p.daily_downloads), "Downloads", move |_| primary);
    chart_box(160.0, chart)
}

/// Chart of the subscriber count over `recorded`, the days of the window with a snapshot. The chart can't leave a day
/// empty, so a day without a snapshot is left off the axis and the line joins the days around it. One day has no area
/// to fill and is drawn as a dot.
fn subscriber_chart(recorded: &[ChartPoint], t: &Tokens) -> Div {
    type Count = (SharedString, i64);
    let counts: Vec<Count> =
        recorded.iter().filter_map(|p| Some((format_snapshot_day(p.taken_on).into(), p.subscribers?))).collect();
    // A scale from 0 to 0 places nothing, so the top is at least 1.
    let top = counts.iter().map(|d| d.1).max().unwrap_or(0).max(1) as f64;
    match counts.len() {
        0 => empty("No snapshots in this period.", t),
        1 => chart_box(
            128.0,
            LineChart::new(counts)
                .x(|d: &Count| d.0.clone())
                .y(|d: &Count| d.1 as f64)
                .y_domain(0.0, top)
                .stroke(t.primary)
                .dot()
                .name("Subscribers")
                .tooltip_value(|d: &Count, _| format_number(d.1).into()),
        ),
        _ => chart_box(
            128.0,
            AreaChart::new(counts)
                .x(|d: &Count| d.0.clone())
                .y(|d: &Count| d.1 as f64)
                .y_domain(0.0, top)
                .stroke(t.primary)
                .fill(alpha(t.primary, 0.35))
                .linear()
                .x_tick_count(X_LABELS)
                .name("Subscribers")
                .tooltip_value(|d: &Count, _, _| format_number(d.1).into()),
        ),
    }
}

/// Bar chart of the daily subscriber change over the same days as [`subscriber_chart`] above it. It draws no dates:
/// the chart puts the date of a signed bar beside the zero line, where 30 narrow bands let a neighbouring bar run
/// through it. The chart above shows the dates, and the tooltip names the day.
fn signed_change_chart(recorded: &[ChartPoint], t: &Tokens) -> Option<Div> {
    if recorded.iter().all(|p| p.daily_subscriber_change.is_none()) {
        return None;
    }
    let (gain, loss) = (t.primary, t.destructive);
    let color = move |v: Option<i64>| if v.is_some_and(|v| v < 0) { loss } else { gain };
    let chart =
        daily_bars("stats-change", days(recorded, |p| p.daily_subscriber_change), "Change", color).band_tick_count(0);
    Some(chart_box(96.0, chart))
}

fn ratings_panel(ratings: &AddonRatingStats, t: &Tokens) -> Div {
    let total = ratings.total_votes;
    let plural = |n: i64| if n == 1 { "" } else { "s" };
    let body = if total == 0 {
        vec![empty("No ratings yet.", t).into_any_element()]
    } else {
        let like_frac = (ratings.likes as f32 / total as f32).clamp(0.0, 1.0);
        let dislike_frac = (ratings.dislikes as f32 / total as f32).clamp(0.0, 1.0);
        let bar = div()
            .flex()
            .h(px(8.0))
            .overflow_hidden()
            .rounded_full()
            .bg(t.muted)
            .child(
                div()
                    .h_full()
                    .w(relative(like_frac))
                    .bg(t.primary)
                    .rounded_l_full()
                    .when(like_frac >= 0.999, |d| d.rounded_r_full()),
            )
            .child(
                div()
                    .h_full()
                    .w(relative(dislike_frac))
                    .bg(alpha(t.muted_foreground, 0.35))
                    .rounded_r_full()
                    .when(like_frac <= 0.001, |d| d.rounded_l_full()),
            );
        let summary = format!(
            "{} like{} · {} dislike{} · {}",
            format_number(ratings.likes),
            plural(ratings.likes),
            format_number(ratings.dislikes),
            plural(ratings.dislikes),
            format_ratio(ratings.like_percentage.unwrap_or(0.0)),
        );
        let mut v = vec![
            bar.into_any_element(),
            div().text_sm_tw().text_color(t.muted_foreground).child(summary).into_any_element(),
        ];
        if ratings.deleted_accounts > 0 {
            v.push(
                div()
                    .text_xs_tw()
                    .text_color(t.muted_foreground)
                    .child(format!(
                        "Includes {} vote{} from deleted accounts.",
                        format_number(ratings.deleted_accounts),
                        plural(ratings.deleted_accounts)
                    ))
                    .into_any_element(),
            );
        }
        v
    };
    div().flex().flex_col().gap_3().child(heading("Ratings")).children(body)
}

fn version_panel(versions: &AddonVersionStats, t: &Tokens) -> Div {
    let bucket = |b: VersionLagBucket| versions.buckets.count(b);
    let bucket_total: i64 = LAG_ORDER.iter().map(|b| bucket(*b)).sum();
    let panel = div().flex().flex_col().gap_3().child(heading("Version adoption"));
    if bucket_total == 0 {
        return panel.child(empty("No subscribers yet.", t));
    }
    let legend = div().flex().flex_wrap().gap_x_4().gap_y_1().text_xs_tw().text_color(t.muted_foreground).children(
        LAG_ORDER.iter().map(|b| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().flex_none().size_2().rounded(px(2.0)).bg(lag_color(*b, t)))
                .child(lag_label(*b))
                .child(div().text_color(t.foreground).child(format_number(bucket(*b))))
        }),
    );
    let bars: Vec<VersionBar> = versions
        .share
        .iter()
        .map(|r| VersionBar {
            release_id: r.release_id,
            tag: r.tag_name.clone().into(),
            subscribers: r.subscribers,
            share: r.share,
            bucket: r.lag_bucket,
            color: lag_color(r.lag_bucket, t),
        })
        .collect();
    let height = (bars.len() as f32 * 36.0).max(80.0);
    let tags = version_tags(&bars, height, t);
    let chart = BarChart::new(bars)
        .alignment(BarAlignment::Left)
        .band(|b: &VersionBar| VersionBand(b.release_id, b.tag.clone()))
        .value(|b: &VersionBar| b.subscribers as f64)
        .fill(|b: &VersionBar, _, _, _| b.color)
        .corner_radii(Corners::all(px(4.0)))
        .padding_inner(BAR_GAP)
        .padding_outer(VERSION_OUTER_GAP)
        .max_band_width(px(VERSION_BAR_MAX))
        .label_axis(false)
        .grid(false)
        .name("Subscribers")
        .tooltip_title(|b: &VersionBar| format!("{} · {}", b.tag, lag_label(b.bucket)).into())
        .tooltip_value(|b: &VersionBar, _| {
            format!("{} ({})", format_number(b.subscribers), format_ratio(b.share)).into()
        });
    let row = div().flex().gap_3().child(tags).child(div().flex_1().min_w_0().h(px(height)).child(chart));
    panel.child(legend).child(row)
}

// ---------------------------------------------------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------------------------------------------------

pub struct AddonStatistics {
    uuid: String,
    alias: String,
    queries: Entity<DevQueries>,
    /// Number of days the charts show, 7 or 30.
    period: u32,
    _observe: Subscription,
}

impl AddonStatistics {
    pub fn new(uuid: String, alias: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let queries = DevQueries::global(cx);
        queries.update(cx, |q, cx| q.ensure_stats(&uuid, cx));
        let observe = cx.observe(&queries, |_, _, cx| cx.notify());
        Self { uuid, alias, queries, period: 7, _observe: observe }
    }
}

impl Render for AddonStatistics {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let loaded = self.queries.read(cx).stats(&self.uuid, cx);

        if loaded.loading {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .py_16()
                .text_color(t.muted_foreground)
                .child(div().opacity(0.5).child(spinner().with_size(px(32.0))))
                .child(div().mt_3().text_sm_tw().child("Loading statistics..."))
                .into_any_element();
        }
        let Some(stats) = loaded.stats else {
            let uuid = self.uuid.clone();
            let queries = self.queries.clone();
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .py_16()
                .child(
                    div()
                        .text_center()
                        .text_sm_tw()
                        .text_color(t.destructive)
                        .child(loaded.error.unwrap_or_else(|| "These statistics could not be loaded.".to_owned())),
                )
                .child(
                    Button::new("stats-retry")
                        .variant(ButtonVariant::Outline)
                        .label("Retry")
                        .on_click(move |_, _, cx| queries.update(cx, |q, cx| q.refetch_stats(&uuid, cx))),
                )
                .into_any_element();
        };

        let today = Utc::now().date_naive();
        let download_series = series_in_window(&stats.downloads.series, self.period, today);
        let recorded: Vec<ChartPoint> = series_in_window(&stats.subscribers.series, self.period, today)
            .into_iter()
            .filter(|p| p.subscribers.is_some())
            .collect();
        let like_label = stats.ratings.like_percentage.map_or_else(|| "No votes".to_owned(), format_ratio);

        let period_buttons = div().flex().gap_1().children([7u32, 30].map(|days| {
            Button::new(("stats-period", days as usize))
                .size(ButtonSize::Sm)
                .variant(if self.period == days { ButtonVariant::Secondary } else { ButtonVariant::Ghost })
                .label(format!("{days} days"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.period = days;
                    cx.notify();
                }))
        }));

        let download_chart = daily_download_chart(&download_series, &t);
        let subscriber_chart = subscriber_chart(&recorded, &t);
        let change_chart = signed_change_chart(&recorded, &t);
        let ratings = ratings_panel(&stats.ratings, &t);
        let versions = version_panel(&stats.versions, &t);

        let sections: Vec<gpui_kit::AnyElement> = vec![
                div()
                    .flex()
                    .flex_wrap()
                    .gap_8()
                    .border_b_1()
                    .border_color(t.border)
                    .pb_5()
                    .child(metric("Total downloads", format_number(stats.downloads.total), &t))
                    .child(metric("Subscribers", format_number(stats.subscribers.current), &t))
                    .child(metric("Liked", like_label, &t)).into_any_element(),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div().flex().items_center().gap_2().text_sm_tw().font_weight(FontWeight::MEDIUM)
                                    .child(Icon::new(icons::CHART_COLUMN).size(px(16.0)))
                                    .child("Download trend"),
                            )
                            .child(period_buttons),
                    )
                    .child(download_chart)
                    .child(note("Daily downloads. A missing day means no data was recorded, not zero downloads.", &t)).into_any_element(),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(heading("Subscribers"))
                    .child(subscriber_chart)
                    .children(change_chart)
                    .child(note(
                        "Subscriber count over time, with daily gains and losses below. Bars below zero mean more people unsubscribed than subscribed that day.",
                        &t,
                    )).into_any_element(),
                ratings.into_any_element(),
                versions.into_any_element(),
        ];
        div().flex().flex_col().gap_8().children(sections).into_any_element()
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(day: &str, subs: i64, dl: Option<i64>, ch: Option<i64>) -> SnapshotPoint {
        SnapshotPoint {
            taken_on: date(day),
            downloads: 0,
            subscribers: subs,
            daily_downloads: dl,
            daily_subscriber_change: ch,
        }
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn window_fills_missing_days() {
        let series = [snap("2025-10-12", 10, Some(3), Some(1)), snap("2025-10-17", 12, None, Some(-2))];
        let pts = series_in_window(&series, 7, date("2025-10-17"));
        assert_eq!(pts.len(), 7);
        assert_eq!(pts[0].taken_on, date("2025-10-11"));
        assert_eq!(pts[6].taken_on, date("2025-10-17"));
        assert_eq!(
            pts[1],
            ChartPoint {
                taken_on: date("2025-10-12"),
                daily_downloads: Some(3),
                subscribers: Some(10),
                daily_subscriber_change: Some(1)
            }
        );
        assert_eq!(pts[0].subscribers, None);
        assert_eq!(
            pts[2],
            ChartPoint {
                taken_on: date("2025-10-13"),
                daily_downloads: None,
                subscribers: None,
                daily_subscriber_change: None
            }
        );
        assert_eq!(pts[6].daily_downloads, None);
        assert_eq!(pts[6].daily_subscriber_change, Some(-2));
    }

    #[test]
    fn window_is_empty_without_subscribers_and_crosses_months() {
        assert!(series_in_window(&[], 30, date("2025-10-17")).is_empty());
        // out-of-window point only
        assert!(series_in_window(&[snap("2025-01-01", 1, None, None)], 7, date("2025-10-17")).is_empty());
        let pts = series_in_window(&[snap("2025-09-20", 5, None, None)], 30, date("2025-10-17"));
        assert_eq!(pts.len(), 30);
        assert_eq!(pts[0].taken_on, date("2025-09-18"));
        assert_eq!(pts[2].subscribers, Some(5));
    }

    #[test]
    fn day_and_ratio_formatting() {
        assert_eq!(format_snapshot_day(date("2025-10-05")), "Oct 5");
        assert_eq!(format_snapshot_day(date("2025-12-31")), "Dec 31");
        assert_eq!(format_ratio(0.8), "80%");
        assert_eq!(format_ratio(0.285), "28.5%");
        assert_eq!(format_ratio(0.2857), "28.6%");
        assert_eq!(format_ratio(0.0), "0%");
        assert_eq!(format_ratio(1.0), "100%");
        assert_eq!(format_ratio(0.5812), "58.1%");
        assert_eq!(format_ratio(0.9999), "100%");
    }
}
