//! Small cells shared by the list rows and grid cards: download meter, like ring, tag badges, action button and warning
//! icon.
use std::time::Duration;

use cam_ui::{
    components::{Side, Tooltip},
    icons,
    theme::{Tokens, Tw, TwText, alpha, hue_from_string, lerp, lerp_color, oklcha, tw},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, PathBuilder, Styled, TextRun, Window, canvas, div, ease_in_out, point,
    prelude::FluentBuilder as _, px,
};

// ---------------------------------------------------------------------------------------------------------
// numbers
// ---------------------------------------------------------------------------------------------------------

/// Compact English notation with at most one decimal: `999`, `1K`, `1.2K`, `12.3K`, `1.5M`.
pub fn compact_number(n: i64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e3, "K"), (1e6, "M"), (1e9, "B"), (1e12, "T")];
    let abs = n.unsigned_abs() as f64;
    if abs < 1000.0 {
        return n.to_string();
    }
    let mut unit = UNITS.iter().rposition(|(v, _)| abs >= *v).unwrap_or(0);
    let mut scaled = (abs / UNITS[unit].0 * 10.0).round() / 10.0;
    if scaled >= 1000.0 && unit + 1 < UNITS.len() {
        unit += 1;
        scaled = (abs / UNITS[unit].0 * 10.0).round() / 10.0;
    }
    let sign = if n < 0 { "-" } else { "" };
    if scaled.fract() == 0.0 {
        format!("{sign}{}{}", scaled as i64, UNITS[unit].1)
    } else {
        format!("{sign}{scaled:.1}{}", UNITS[unit].1)
    }
}

/// Heights in pixels of the five bars of the download meter.
const BAR_HEIGHTS: [f32; 5] = [4.0, 6.0, 8.0, 10.0, 12.0];

/// Number of filled bars (1 to 5) of the download meter, on a log scale relative to the most downloaded addon.
pub fn download_level(downloads: i64, max_downloads: i64) -> usize {
    if downloads <= 1 || max_downloads <= 1 {
        return 1;
    }
    let level = (1.0 + 4.0 * (downloads as f64).ln() / (max_downloads as f64).ln()).round();
    (level as i64).clamp(1, BAR_HEIGHTS.len() as i64) as usize
}

/// Colour of the like ring: green from 90, the primary colour from 70, amber below.
pub fn ring_color(percentage: i64, t: &Tokens) -> gpui_kit::Hsla {
    if percentage >= 90 {
        tw(Tw::Green, 500)
    } else if percentage >= 70 {
        t.primary
    } else {
        tw(Tw::Amber, 500)
    }
}

// ---------------------------------------------------------------------------------------------------------
// download meter / like ring
// ---------------------------------------------------------------------------------------------------------

/// Compact download count followed by 1 to 5 bars. Fills its parent's width, with the content right-aligned.
pub fn download_meter(downloads: i64, max_downloads: i64, t: &Tokens) -> impl IntoElement {
    let level = download_level(downloads, max_downloads);
    div()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(8.0))
        .text_xs_tw()
        .child(div().font_mono_tw().text_color(t.foreground).child(compact_number(downloads)))
        .child(div().flex().h(px(12.0)).items_end().gap(px(2.0)).children(BAR_HEIGHTS.iter().enumerate().map(
            |(i, h)| {
                div().w(px(3.0)).h(px(*h)).rounded(px(1.0)).bg(if i < level { t.muted_foreground } else { t.border })
            },
        )))
}

/// Like ratio: an 18px progress ring with the percentage, or a dash without ratings. `fixed_width` reserves room for
/// three digits so the rings line up down a column.
pub fn like_ring(percentage: Option<i64>, fixed_width: bool, t: &Tokens) -> impl IntoElement {
    let Some(pct) = percentage else {
        return div()
            .flex()
            .justify_end()
            .font_mono_tw()
            .text_xs_tw()
            .text_color(alpha(t.muted_foreground, 0.6))
            .child("—")
            .into_any_element();
    };
    let color = ring_color(pct, t);
    let track = t.border;
    // The ring geometry is laid out on a 20px grid but drawn at 18px, hence the 0.9 scale.
    let ring = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let scale = 0.9;
            let stroke = px(2.5 * scale);
            let centre = point(bounds.origin.x + px(9.0), bounds.origin.y + px(9.0));
            let radius = 7.5 * scale;
            let top = point(centre.x, centre.y - px(radius));
            let bottom = point(centre.x, centre.y + px(radius));
            let radii = point(px(radius), px(radius));

            let mut track_path = PathBuilder::stroke(stroke);
            track_path.move_to(top);
            track_path.arc_to(radii, px(0.0), false, true, bottom);
            track_path.arc_to(radii, px(0.0), false, true, top);
            if let Ok(path) = track_path.build() {
                window.paint_path(path, track);
            }

            let fraction = (pct as f32 / 100.0).clamp(0.0, 1.0);
            if fraction <= 0.0 {
                return;
            }
            let mut arc = PathBuilder::stroke(stroke);
            arc.move_to(top);
            if fraction >= 0.999 {
                arc.arc_to(radii, px(0.0), false, true, bottom);
                arc.arc_to(radii, px(0.0), false, true, top);
            } else {
                let theta = fraction * std::f32::consts::TAU;
                let end = point(centre.x + px(radius * theta.sin()), centre.y - px(radius * theta.cos()));
                arc.arc_to(radii, px(0.0), theta > std::f32::consts::PI, true, end);
            }
            if let Ok(path) = arc.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(18.0))
    .flex_none();

    div()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(6.0))
        .text_xs_tw()
        .child(ring)
        .child(
            div().font_mono_tw().text_color(t.foreground).when(fixed_width, |d| d.w(px(21.6))).child(pct.to_string()),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------------------------------------
// tag badges
// ---------------------------------------------------------------------------------------------------------

/// Up to `max_tags` tags as outlined badges tinted by a hue derived from the tag name, plus a `+N` count for the rest.
pub fn tag_dots(tags: &[String], max_tags: usize, t: &Tokens) -> Option<impl IntoElement> {
    if tags.is_empty() {
        return None;
    }
    let visible = &tags[..tags.len().min(max_tags)];
    let hidden = tags.len() - visible.len();
    let muted = t.muted_foreground;
    Some(
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_xs_tw()
            .text_color(muted)
            .children(visible.iter().map(|tag| {
                let hue = hue_from_string(tag) as f32;
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .rounded(cam_ui::theme::radius::MD)
                    .border_1()
                    .border_color(oklcha(0.72, 0.14, hue, 0.35))
                    .bg(oklcha(0.72, 0.14, hue, 0.12))
                    .px(px(6.0))
                    .py(px(0.0))
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(oklcha(0.82, 0.1, hue, 1.0))
                    .child(tag.clone())
            }))
            .when(hidden > 0, |d| d.child(div().text_color(alpha(muted, 0.6)).child(format!("+{hidden}")))),
    )
}

// ---------------------------------------------------------------------------------------------------------
// action button
// ---------------------------------------------------------------------------------------------------------

/// Duration of the expand / collapse animation.
const ACTION_TRANSITION: Duration = Duration::from_millis(150);

/// Green "Installed" label, or a muted arrow that expands into a "View" pill while the surrounding row / card is
/// hovered.
///
/// gpui applies hover styles only at paint time: a hover-driven width / padding is ignored by layout, and text and
/// icon colours are baked in during layout. The expansion is therefore driven by the row's hover state instead:
/// `hover` is `None` until the row is first hovered, then `Some(hovered)`. Each flip restarts a 150ms animation that
/// re-lays the button out every frame. `key` must be unique per button.
pub fn action_button(
    installed: bool,
    compact: bool,
    hover: Option<bool>,
    key: ElementId,
    window: &mut Window,
    t: &Tokens,
) -> AnyElement {
    let height = if compact { 24.0 } else { 32.0 };
    let icon_size = if compact { 14.0 } else { 16.0 };
    let text_size = if compact { 11.0 } else { 12.0 };
    let line_height = if compact { 16.5 } else { 16.0 };

    if installed {
        return div()
            .flex()
            .items_center()
            .h(px(height))
            .gap(px(if compact { 4.0 } else { 6.0 }))
            .when(!compact, |d| d.px(px(8.0)))
            .text_size(px(text_size))
            .line_height(px(line_height))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tw(Tw::Green, 500))
            .child(Icon::new(icons::CHECK).size(px(icon_size)))
            .child("Installed")
            .into_any_element();
    }

    // The label is clipped to its natural width, so measure it and let the animated width end exactly there.
    let mut font = window.text_style().font();
    font.weight = FontWeight::SEMIBOLD;
    let label_width = window
        .text_system()
        .shape_line(
            "View".into(),
            px(text_size),
            &[TextRun {
                len: "View".len(),
                font,
                color: t.primary_foreground,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        )
        .width
        .as_f32()
        .ceil();

    let primary = t.primary;
    let primary_fg = t.primary_foreground;
    let idle_fg = alpha(t.muted_foreground, 0.7);
    let radius = if compact { cam_ui::theme::radius::MD } else { cam_ui::theme::radius::LG };
    let (px_idle, px_hover) = if compact { (4.0, 8.0) } else { (8.0, 12.0) };

    // `p`: 0 = collapsed arrow, 1 = expanded "View" pill.
    let build = move |el: Div, p: f32| -> Div {
        el.flex()
            .items_center()
            .h(px(height))
            .rounded(radius)
            .px(px(lerp(px_idle, px_hover, p)))
            .bg(alpha(primary, p))
            .text_size(px(text_size))
            .line_height(px(line_height))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(lerp_color(idle_fg, primary_fg, p))
            .child(
                div()
                    .w(px(label_width * p))
                    .mr(px(6.0 * p))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .opacity(p)
                    .child("View"),
            )
            .child(Icon::new(icons::ARROW_RIGHT).size(px(icon_size)))
    };

    match hover {
        None => build(div(), 0.0).into_any_element(),
        Some(hovered) => div()
            .with_animation(
                (key, if hovered { "expand" } else { "collapse" }),
                Animation::new(ACTION_TRANSITION).with_easing(ease_in_out),
                move |el, t| build(el, if hovered { t } else { 1.0 - t }),
            )
            .into_any_element(),
    }
}

// ---------------------------------------------------------------------------------------------------------
// warning icon
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WarningStyle {
    /// Labelled amber "CAUTION" chip (list rows).
    Chip,
    /// Bare 14px icon (grid cards).
    Compact,
}

/// Amber warning triangle with the warning text as a tooltip. Swallows clicks so the row does not open.
pub fn warning_icon(warning: Option<&str>, style: WarningStyle, id: gpui_kit::ElementId) -> Option<impl IntoElement> {
    let warning = warning.filter(|w| !w.is_empty())?;
    let amber = tw(Tw::Amber, 500);
    let trigger = match style {
        WarningStyle::Chip => div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.0))
            .rounded(cam_ui::theme::radius::SM)
            .bg(alpha(amber, 0.15))
            .px(px(6.0))
            .py(px(2.0))
            .text_size(px(10.0))
            .line_height(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(amber)
            .child(Icon::new(icons::TRIANGLE_ALERT).size(px(10.0)))
            .child("CAUTION"),
        WarningStyle::Compact => {
            div().flex_none().text_color(amber).child(Icon::new(icons::TRIANGLE_ALERT).size(px(14.0)))
        }
    }
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation());
    Some(Tooltip::new(warning.to_string()).id(id).max_width(px(320.0)).side(Side::Top).flex_none().child(trigger))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_number_matches_intl() {
        assert_eq!(compact_number(0), "0");
        assert_eq!(compact_number(999), "999");
        assert_eq!(compact_number(1000), "1K");
        assert_eq!(compact_number(1234), "1.2K");
        assert_eq!(compact_number(12345), "12.3K");
        assert_eq!(compact_number(123456), "123.5K");
        assert_eq!(compact_number(999_949), "999.9K");
        assert_eq!(compact_number(999_950), "1M");
        assert_eq!(compact_number(1_500_000), "1.5M");
        assert_eq!(compact_number(2_000_000_000), "2B");
    }

    #[test]
    fn download_level_is_log_scaled() {
        assert_eq!(download_level(0, 1000), 1);
        assert_eq!(download_level(1, 1000), 1);
        assert_eq!(download_level(50, 1), 1);
        assert_eq!(download_level(1000, 1000), 5);
        assert_eq!(download_level(32, 1000), 3); // 1 + 4*ln(32)/ln(1000) = 3.0
        assert_eq!(download_level(2000, 1000), 5); // clamped
    }

    #[test]
    fn ring_colour_thresholds() {
        let t = Tokens::dark(crate::ui::accent(Default::default()));
        assert_eq!(ring_color(100, &t), tw(Tw::Green, 500));
        assert_eq!(ring_color(90, &t), tw(Tw::Green, 500));
        assert_eq!(ring_color(89, &t), t.primary);
        assert_eq!(ring_color(70, &t), t.primary);
        assert_eq!(ring_color(69, &t), tw(Tw::Amber, 500));
    }
}
