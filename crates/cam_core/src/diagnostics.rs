use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use memchr::memmem;
use regex::{CaptureLocations, Regex};
use serde::{Deserialize, Serialize};

const MAX_LOG_TAIL_BYTES: u64 = 8 << 20;
const MAX_LOG_LINE_BYTES: usize = 64 << 10;
const MAX_LOG_UNIQUE_ERRORS: usize = 5000;

/// One group of identical errors. Fields serialize as-is in PascalCase (`Type`, `Addon`, `File`, `Error`, `Count`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LogParseResult {
    pub r#type: String,
    pub addon: String,
    pub file: String,
    pub error: String,
    pub count: i64,
}

struct Pattern {
    regex: Regex,
    err_type: &'static str,
    /// `Some` => this constant, otherwise the first path segment of the file.
    constant_addon: Option<&'static str>,
}

/// Every pattern contains this literal, so a line without it cannot match.
const PATTERN_LITERAL: &[u8] = b"Error]";

static PATTERNS: LazyLock<[Pattern; 3]> = LazyLock::new(|| {
    [
        Pattern {
            regex: Regex::new(r"\[Lua Error].*/Addon/(.+\.lua:\d+): (.+)").unwrap(),
            err_type: "lua",
            constant_addon: None,
        },
        Pattern {
            regex: Regex::new(r"/Addon/([^:]+:\d+):\s*\[Script Error]\s*(.*)").unwrap(),
            err_type: "addon",
            constant_addon: None,
        },
        Pattern {
            regex: Regex::new(r"@.*?/x2ui/addons/([^:]+:\d+):\s*\[Script Error]\s*(.*)").unwrap(),
            err_type: "api",
            constant_addon: Some("x2ui"),
        },
    ]
});

pub fn parse_log_path(path: impl AsRef<Path>) -> std::io::Result<Vec<LogParseResult>> {
    let mut f = std::fs::File::open(path)?;
    let mut skip_first = false;
    let size = f.metadata()?.len();
    if size > MAX_LOG_TAIL_BYTES {
        f.seek(SeekFrom::Start(size - MAX_LOG_TAIL_BYTES - 1))?;
        skip_first = true;
    }
    parse_log(f, skip_first)
}

pub fn parse_log<R: Read>(r: R, mut skip_first: bool) -> std::io::Result<Vec<LogParseResult>> {
    /// How often an error occurred, and the number of the matching line it was last seen on.
    struct Seen {
        count: i64,
        last_seen: usize,
    }
    let mut br = BufReader::with_capacity(MAX_LOG_LINE_BYTES, r);
    // Keyed by (pattern index, file, error). The key strings become the result's strings.
    let mut seen: HashMap<(usize, String, String), Seen> = HashMap::new();
    // Capture slots for each pattern, reused for every line. Made on the first candidate line.
    let mut locations: Option<[CaptureLocations; 3]> = None;
    let literal = memmem::Finder::new(PATTERN_LITERAL);
    let mut seq = 0usize;
    let mut buf: Vec<u8> = Vec::new();

    loop {
        buf.clear();
        let eof = !read_line_capped(&mut br, &mut buf)?;
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }

        if skip_first {
            skip_first = false;
        } else if literal.find(&buf).is_some() {
            // Lossy conversion only inserts U+FFFD, never ASCII, so the prefilter above cannot reject a match.
            let line = String::from_utf8_lossy(&buf);
            let locations =
                locations.get_or_insert_with(|| PATTERNS.each_ref().map(|p| p.regex.capture_locations()));
            for (i, (p, slots)) in PATTERNS.iter().zip(locations).enumerate() {
                if p.regex.captures_read(slots, &line).is_none() {
                    continue;
                }
                seq += 1;
                // Both groups take part in every match of every pattern.
                let group = |g| slots.get(g).map_or("", |(start, end)| &line[start..end]);
                let entry =
                    seen.entry((i, group(1).to_owned(), group(2).to_owned())).or_insert(Seen { count: 0, last_seen: 0 });
                entry.count += 1;
                entry.last_seen = seq;
                break;
            }
        }
        if eof {
            break;
        }
    }

    let mut ordered: Vec<_> = seen.into_iter().collect();
    // `last_seen` is unique per entry, so an unstable sort gives the same order as a stable one.
    ordered.sort_unstable_by_key(|(_, s)| s.last_seen);
    let dropped = ordered.len().saturating_sub(MAX_LOG_UNIQUE_ERRORS);
    Ok(ordered
        .into_iter()
        .skip(dropped)
        .map(|((i, file, error), s)| {
            let p = &PATTERNS[i];
            LogParseResult {
                r#type: p.err_type.to_owned(),
                addon: p.constant_addon.unwrap_or_else(|| file.split('/').next().unwrap_or("")).to_owned(),
                file,
                error,
                count: s.count,
            }
        })
        .collect())
}

/// Reads one line into `buf` without its `\n`, keeping at most `MAX_LOG_LINE_BYTES` and skipping the rest of a longer
/// line. Returns false when the input ended before a `\n`.
fn read_line_capped(r: &mut impl BufRead, buf: &mut Vec<u8>) -> std::io::Result<bool> {
    loop {
        let chunk = match r.fill_buf() {
            Ok(chunk) => chunk,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            return Ok(false);
        }
        let newline = memchr::memchr(b'\n', chunk);
        let line = &chunk[..newline.unwrap_or(chunk.len())];
        let room = MAX_LOG_LINE_BYTES - buf.len();
        buf.extend_from_slice(&line[..line.len().min(room)]);
        let consumed = newline.map_or(chunk.len(), |i| i + 1);
        r.consume(consumed);
        if newline.is_some() {
            return Ok(true);
        }
    }
}

/// The addon errors in `ArcheAge.log` in the ArcheAge Classic Documents folder `aac_dir`.
pub fn diagnose_issues(aac_dir: &Path) -> Result<Vec<LogParseResult>> {
    Ok(parse_log_path(aac_dir.join("ArcheAge.log"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(s: &str) -> Vec<LogParseResult> {
        parse_log(Cursor::new(s.as_bytes().to_vec()), false).unwrap()
    }

    #[test]
    fn error_types() {
        let log = "[Lua Error] x C:/Game/Addon/foo/main.lua:12: attempt to index nil\n\
                   /Addon/bar/init.lua:3: [Script Error] bad thing\n\
                   @C:/x/x2ui/addons/ui.lua:9: [Script Error] api broke\n\
                   unrelated\n";
        let r = parse(log);
        assert_eq!(r.len(), 3);
        assert_eq!((r[0].r#type.as_str(), r[0].addon.as_str(), r[0].file.as_str()), ("lua", "foo", "foo/main.lua:12"));
        assert_eq!(r[0].error, "attempt to index nil");
        assert_eq!((r[1].r#type.as_str(), r[1].addon.as_str()), ("addon", "bar"));
        assert_eq!((r[2].r#type.as_str(), r[2].addon.as_str()), ("api", "x2ui"));
    }

    #[test]
    fn merges_repeats_and_orders_by_latest() {
        let a = "/Addon/a/x.lua:1: [Script Error] A\n";
        let b = "/Addon/b/x.lua:1: [Script Error] B\n";
        let r = parse(&format!("{a}{b}{a}"));
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].addon.as_str(), r[0].count), ("b", 1));
        assert_eq!((r[1].addon.as_str(), r[1].count), ("a", 2));
    }

    #[test]
    fn crlf_and_no_trailing_newline() {
        let r = parse("/Addon/a/x.lua:1: [Script Error] A\r\n/Addon/a/x.lua:1: [Script Error] A");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].count, 2);
        assert_eq!(r[0].error, "A");
    }

    #[test]
    fn overlong_line_is_truncated_not_fatal() {
        // The error past the cut belongs to the overlong line and must not be read as a line of its own.
        let long = format!(
            "/Addon/a/x.lua:1: [Script Error] {}/Addon/c/x.lua:1: [Script Error] C\n",
            "x".repeat(200_000)
        );
        let r = parse(&format!("{long}/Addon/b/x.lua:1: [Script Error] B\n"));
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].addon, "a");
        assert!(r[0].error.len() < MAX_LOG_LINE_BYTES);
        assert_eq!((r[1].addon.as_str(), r[1].error.as_str()), ("b", "B"));
    }

    #[test]
    fn result_cap_keeps_newest() {
        let mut s = String::new();
        for i in 0..(MAX_LOG_UNIQUE_ERRORS + 10) {
            s.push_str(&format!("/Addon/a/x.lua:1: [Script Error] e{i}\n"));
        }
        let r = parse(&s);
        assert_eq!(r.len(), MAX_LOG_UNIQUE_ERRORS);
        assert_eq!(r[0].error, "e10");
    }

    #[test]
    fn tail_boundary_drops_partial_first_line() {
        let dir = std::env::temp_dir().join(format!("cam-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("ArcheAge.log");
        let mut data = String::from("/Addon/old/x.lua:1: [Script Error] partial\n");
        let filler = "z".repeat(1000) + "\n";
        while (data.len() as u64) < MAX_LOG_TAIL_BYTES + 100 {
            data.push_str(&filler);
        }
        data.push_str("/Addon/new/x.lua:1: [Script Error] fresh\n");
        std::fs::write(&p, &data).unwrap();
        let r = parse_log_path(&p).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].addon, "new");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
