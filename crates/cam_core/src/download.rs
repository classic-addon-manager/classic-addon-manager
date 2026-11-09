use std::fmt;
use std::io::{self, Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::api;
use crate::throttle::Throttle;

/// The download did not match its published size or SHA-256 and was discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationFailed(pub String);

impl fmt::Display for VerificationFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "download verification failed: {}", self.0)
    }
}

impl std::error::Error for VerificationFailed {}

/// Size and SHA-256 (hex) a download must match.
pub struct ExpectedDownload<'a> {
    pub sha256: &'a str,
    pub size: u64,
}

/// Downloads `url` to `path` through a `.part` temp file next to it, so an existing target survives failures.
pub fn download_file(url: &str, path: impl AsRef<Path>) -> Result<()> {
    download(url, path.as_ref(), None, &mut |_| {})
}

/// Like [`download_file`], but keeps the file only if it is exactly `expected.size` bytes with SHA-256
/// `expected.sha256`. Stops as soon as more bytes arrive than expected. `progress(received)` runs once at the start,
/// at most every 50 ms while downloading, and once at the end.
pub fn download_verified(
    url: &str,
    path: impl AsRef<Path>,
    expected: &ExpectedDownload,
    progress: &mut dyn FnMut(u64),
) -> Result<()> {
    download(url, path.as_ref(), Some(expected), progress)
}

fn download(url: &str, path: &Path, expected: Option<&ExpectedDownload>, progress: &mut dyn FnMut(u64)) -> Result<()> {
    let mut reader = api::download(url).with_context(|| format!("downloading {url}"))?;

    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let prefix = format!("{}.", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default());
    // Dropping the part file on any error below deletes it.
    let mut tmp = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".part")
        .tempfile_in(dir)
        .with_context(|| format!("creating a temporary file in {}", dir.display()))?;

    copy_checked(url, &mut reader, &mut tmp, expected, progress)?;
    tmp.flush().with_context(|| format!("closing {}", tmp.path().display()))?;
    tmp.persist(path).with_context(|| format!("replacing {}", path.display()))?;

    log::info!("Downloaded {}", path.display());
    Ok(())
}

/// Copies `reader` into `out` and checks the size and SHA-256 against `expected` as it goes.
fn copy_checked(
    url: &str,
    reader: &mut impl Read,
    out: &mut impl Write,
    expected: Option<&ExpectedDownload>,
    progress: &mut dyn FnMut(u64),
) -> Result<()> {
    let mut hasher = expected.map(|_| Sha256::new());
    let mut buf = [0u8; 64 * 1024];
    let mut received = 0u64;
    let mut throttle = Throttle::new();
    progress(0);
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e).with_context(|| format!("downloading {url}")),
        };
        received += n as u64;
        if let Some(exp) = expected.filter(|exp| received > exp.size) {
            return Err(mismatch(format!("size mismatch (expected {} bytes, received more)", exp.size)));
        }
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&buf[..n]);
        }
        out.write_all(&buf[..n]).with_context(|| format!("downloading {url}"))?;
        if throttle.ready() {
            progress(received);
        }
    }
    progress(received);

    let (Some(exp), Some(hasher)) = (expected, hasher) else {
        return Ok(());
    };
    if received != exp.size {
        return Err(mismatch(format!("size mismatch (expected {} bytes, received {received})", exp.size)));
    }
    let actual = hex::encode(hasher.finalize());
    let wanted = exp.sha256.trim();
    if !actual.eq_ignore_ascii_case(wanted) {
        return Err(mismatch(format!(
            "SHA-256 mismatch (expected {}…, got {}…)",
            wanted.get(..8).unwrap_or(wanted).to_lowercase(),
            &actual[..8]
        )));
    }
    Ok(())
}

fn mismatch(detail: String) -> anyhow::Error {
    VerificationFailed(detail).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-256 of `b"hello"`.
    const HELLO: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn copy(body: &[u8], sha256: &str, size: u64) -> (Result<()>, Vec<u8>, Vec<u64>) {
        let (mut out, mut seen) = (Vec::new(), Vec::new());
        let res = copy_checked("u", &mut &body[..], &mut out, Some(&ExpectedDownload { sha256, size }), &mut |n| {
            seen.push(n)
        });
        (res, out, seen)
    }

    #[test]
    fn matching_archive_passes_and_reports_final_size() {
        let (res, out, seen) = copy(b"hello", &HELLO.to_uppercase(), 5);
        assert!(res.is_ok());
        assert_eq!(out, b"hello");
        assert_eq!((seen.first(), seen.last()), (Some(&0), Some(&5)));
    }

    fn verification_detail(res: Result<()>) -> String {
        res.unwrap_err().downcast::<VerificationFailed>().expect("a verification failure").0
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let detail = verification_detail(copy(b"hellp", HELLO, 5).0);
        assert!(detail.contains("expected 2cf24dba…"), "{detail}");
    }

    #[test]
    fn short_body_is_rejected() {
        let detail = verification_detail(copy(b"hell", HELLO, 5).0);
        assert!(detail.contains("expected 5 bytes, received 4"), "{detail}");
    }

    #[test]
    fn oversized_body_stops_before_writing_the_excess() {
        let (res, out, _) = copy(b"hello world", HELLO, 5);
        assert!(verification_detail(res).contains("received more"));
        assert!(out.is_empty());
    }

    #[test]
    fn unverified_copy_accepts_anything() {
        let mut out = Vec::new();
        assert!(copy_checked("u", &mut &b"abc"[..], &mut out, None, &mut |_| {}).is_ok());
        assert_eq!(out, b"abc");
    }
}
