use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use regex::Regex;
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
    /// `None` => constant "x2ui", otherwise first path segment.
    constant_addon: Option<&'static str>,
}

fn patterns() -> &'static [Pattern] {
    static P: OnceLock<Vec<Pattern>> = OnceLock::new();
    P.get_or_init(|| {
        vec![
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
    })
}

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
    struct Entry {
        result: LogParseResult,
        last_seen: usize,
    }
    let mut br = BufReader::with_capacity(MAX_LOG_LINE_BYTES, r);
    let mut entries: std::collections::HashMap<(&'static str, String, String), Entry> = Default::default();
    let mut seq = 0usize;
    let mut buf: Vec<u8> = Vec::new();

    loop {
        buf.clear();
        let n = br.read_until(b'\n', &mut buf)?;
        let eof = n == 0 || buf.last() != Some(&b'\n');
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        buf.truncate(MAX_LOG_LINE_BYTES);
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }

        if skip_first {
            skip_first = false;
        } else if !buf.is_empty() {
            let line = String::from_utf8_lossy(&buf);
            for p in patterns() {
                if let Some(caps) = p.regex.captures(&line) {
                    seq += 1;
                    let file = caps[1].to_string();
                    let error = caps[2].to_string();
                    let key = (p.err_type, file.clone(), error.clone());
                    match entries.get_mut(&key) {
                        Some(e) => {
                            e.result.count += 1;
                            e.last_seen = seq;
                        }
                        None => {
                            let addon = match p.constant_addon {
                                Some(a) => a.to_string(),
                                None => file.split('/').next().unwrap_or("").to_string(),
                            };
                            entries.insert(
                                key,
                                Entry {
                                    result: LogParseResult {
                                        r#type: p.err_type.to_string(),
                                        addon,
                                        file,
                                        error,
                                        count: 1,
                                    },
                                    last_seen: seq,
                                },
                            );
                        }
                    }
                    break;
                }
            }
        }
        if eof {
            break;
        }
    }

    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut ordered: Vec<Entry> = entries.into_values().collect();
    ordered.sort_by_key(|e| e.last_seen);
    if ordered.len() > MAX_LOG_UNIQUE_ERRORS {
        ordered.drain(..ordered.len() - MAX_LOG_UNIQUE_ERRORS);
    }
    Ok(ordered.into_iter().map(|e| e.result).collect())
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
        let long = format!("/Addon/a/x.lua:1: [Script Error] {}\n", "x".repeat(200_000));
        let r = parse(&long);
        assert_eq!(r.len(), 1);
        assert!(r[0].error.len() < MAX_LOG_LINE_BYTES);
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
