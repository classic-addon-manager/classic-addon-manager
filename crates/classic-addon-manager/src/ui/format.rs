//! How pages write dates, times, numbers and shortened text.
use std::sync::LazyLock;

use chrono::{DateTime, Local, Utc};

/// Date and time in the local time zone, e.g. `05 Jul 2025 04:31 PM`, or `05 Jul 2025 16:31` when the OS clock is
/// 24-hour (see [`clock_format`]). `short_month` uses `Jul` instead of `July`.
pub fn format_to_local_time(date: &DateTime<Utc>, short_month: bool) -> String {
    let local = date.with_timezone(&Local);
    let month = if short_month { "%b" } else { "%B" };
    local.format(&format!("%d {month} %Y {}", clock_format())).to_string()
}

/// strftime hour/minute pattern matching the user's OS clock preference: `%H:%M` or `%I:%M %p`.
/// Read once per process, changing the OS setting takes effect on restart.
pub fn clock_format() -> &'static str {
    static FORMAT: LazyLock<&str> = LazyLock::new(|| if system_uses_24h() { "%H:%M" } else { "%I:%M %p" });
    *FORMAT
}

/// Windows: the user's short time pattern from Region settings (`HH:mm` vs `h:mm tt`).
#[cfg(windows)]
fn system_uses_24h() -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocaleInfoEx(locale_name: *const u16, lc_type: u32, data: *mut u16, cch_data: i32) -> i32;
    }
    const LOCALE_STIMEFORMAT: u32 = 0x1003;
    // Documented maximum for LOCALE_STIMEFORMAT is 80 characters including the terminator.
    let mut buf = [0u16; 80];
    // SAFETY: a null locale name selects LOCALE_NAME_USER_DEFAULT, the length is the buffer size in u16 units.
    let len = unsafe { GetLocaleInfoEx(std::ptr::null(), LOCALE_STIMEFORMAT, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return false;
    }
    windows_pattern_is_24h(&String::from_utf16_lossy(&buf[..len as usize - 1]))
}

/// Linux: `LC_TIME`'s `T_FMT` from the environment (`LC_ALL` > `LC_TIME` > `LANG`), without touching the
/// process-global locale.
#[cfg(target_os = "linux")]
fn system_uses_24h() -> bool {
    // SAFETY: `newlocale` gets a NUL-terminated name and a null base, the returned handle is checked, only read
    // via `nl_langinfo_l` while alive, and freed once. `nl_langinfo_l` returns a NUL-terminated string owned by it.
    unsafe {
        let loc = libc::newlocale(libc::LC_TIME_MASK, c"".as_ptr(), std::ptr::null_mut());
        if loc.is_null() {
            return true; // POSIX "C" locale: `%H:%M:%S`
        }
        let fmt = std::ffi::CStr::from_ptr(libc::nl_langinfo_l(libc::T_FMT, loc)).to_string_lossy().into_owned();
        libc::freelocale(loc);
        strftime_pattern_is_24h(&fmt)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn system_uses_24h() -> bool {
    false
}

/// Windows time picture: `H` is the 24-hour field, `h`/`tt` the 12-hour ones, `'...'` is literal text.
#[cfg(any(windows, test))]
fn windows_pattern_is_24h(pattern: &str) -> bool {
    let mut quoted = false;
    for c in pattern.chars() {
        match c {
            '\'' => quoted = !quoted,
            'H' if !quoted => return true,
            _ => {}
        }
    }
    false
}

/// strftime pattern: 12-hour iff it uses `%I`, `%l`, `%r` or `%p` (`E`/`O` modifiers allowed).
#[cfg(any(target_os = "linux", test))]
fn strftime_pattern_is_24h(pattern: &str) -> bool {
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        let mut spec = chars.next();
        if matches!(spec, Some('E' | 'O')) {
            spec = chars.next();
        }
        if matches!(spec, Some('I' | 'l' | 'r' | 'p')) {
            return false;
        }
    }
    true
}

/// Local date in `05 Jul 2025` style (day, month, year).
pub fn format_to_local_date(date: &DateTime<Utc>, short_month: bool) -> String {
    let local = date.with_timezone(&Local);
    let month = if short_month { "%b" } else { "%B" };
    local.format(&format!("%d {month} %Y")).to_string()
}

/// Whole days since `date`.
pub fn days_ago(date: &DateTime<Utc>) -> i64 {
    (Utc::now() - *date).num_days()
}

/// Time since `date` as "just now", "5m ago", "2h ago" or "3d ago".
pub fn time_ago(date: &DateTime<Utc>) -> String {
    time_ago_at(date, &Utc::now())
}

fn time_ago_at(date: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let minutes = (*now - *date).num_minutes();
    if minutes < 1 {
        return "just now".into();
    }
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    format!("{}d ago", hours / 24)
}

/// Groups digits in thousands: `12345` -> `12,345`.
pub fn format_number(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// The first `n` characters of `s`, cut on a character boundary (used for error messages in toasts and banners).
pub fn truncate_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(-1234567), "-1,234,567");
    }

    #[test]
    fn clock_pattern_detection() {
        assert!(windows_pattern_is_24h("HH:mm:ss"));
        assert!(!windows_pattern_is_24h("h:mm:ss tt"));
        assert!(!windows_pattern_is_24h("'Heure' h:mm tt"));
        assert!(strftime_pattern_is_24h("%H:%M:%S"));
        assert!(strftime_pattern_is_24h("%T"));
        assert!(!strftime_pattern_is_24h("%r"));
        assert!(!strftime_pattern_is_24h("%OI:%M %p"));
        assert!(strftime_pattern_is_24h("%%I %H"));
    }

    #[test]
    fn relative_time() {
        let now = Utc::now();
        let ago = |d: chrono::Duration| time_ago_at(&(now - d), &now);
        assert_eq!(ago(chrono::Duration::seconds(10)), "just now");
        assert_eq!(ago(chrono::Duration::minutes(5)), "5m ago");
        assert_eq!(ago(chrono::Duration::seconds(3 * 3600 + 5)), "3h ago");
        assert_eq!(ago(chrono::Duration::hours(49)), "2d ago");
        assert_eq!(ago(chrono::Duration::minutes(-2)), "just now");
    }

    #[test]
    fn truncate_is_char_safe() {
        assert_eq!(truncate_chars("héllo", 2), "hé");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }
}
