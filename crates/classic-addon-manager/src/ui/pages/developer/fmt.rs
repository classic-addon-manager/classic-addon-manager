//! Formats the times of the developer API as local dates and times (en-US style), plus number grouping.
use chrono::{DateTime, Local, Utc};

/// Date such as `October 01, 2026`.
pub fn local_date(value: &DateTime<Utc>) -> String {
    value.with_timezone(&Local).format("%B %d, %Y").to_string()
}

/// Date and time such as `October 01 2026 at 04:31 PM` (no comma after the day), or `... at 16:31` when the OS clock
/// is 24-hour.
pub fn local_time(value: &DateTime<Utc>) -> String {
    let pattern = format!("%B %d, %Y at {}", crate::ui::format::clock_format());
    value.with_timezone(&Local).format(&pattern).to_string().replacen(',', "", 1)
}

/// `n` with thousands separators.
pub fn number(n: i64) -> String {
    crate::ui::format::format_number(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_shapes() {
        let at = DateTime::parse_from_rfc3339("2026-10-01T15:38:38Z").unwrap().with_timezone(&Utc);
        assert!(local_date(&at).contains("2026"));
        assert!(!local_time(&at).contains(','));
    }
}
