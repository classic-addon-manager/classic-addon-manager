//! Disk cache for HTTP responses whose bytes never change (the addon icons on the CDN), used by
//! [`crate::http_client`].
//!
//! Each response is one file `<key>.camhttp`: an 8-byte magic, the SHA-256 of the body, then the body. It is written
//! to a temporary file in the same folder and renamed into place, and a record that doesn't match its digest is a
//! miss, so a torn or damaged file is fetched again (and replaced). The oldest records are deleted first when the
//! folder grows past its cap, files the cache didn't name are never touched. Every failure is a miss or a skipped
//! write: a response never depends on the disk.
use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"CAMHTTP1";
const HEADER_LEN: usize = MAGIC.len() + 32;
/// Larger responses are not stored (ureq's default body limit is 10 MB, so they don't arrive anyway).
pub(crate) const MAX_BODY: u64 = 10 * 1024 * 1024;
const RECORD_EXTENSION: &str = "camhttp";
const TEMP_PREFIX: &str = ".camhttp-";
/// A temporary file this old belongs to a write that never finished.
const STALE_TEMP: Duration = Duration::from_secs(60 * 60);

pub struct DiskCache {
    dir: PathBuf,
    key: fn(&str) -> Option<&str>,
    cap: u64,
    /// Bytes of the records on disk: the last scan plus the writes since. Approximate, it only decides when to prune.
    size: AtomicU64,
    /// One pruning pass at a time.
    pruning: Mutex<()>,
}

impl DiskCache {
    /// A cache in `dir` (created on the first write) for the URLs `key` maps to a file name stem, kept under `cap`
    /// bytes. The folder is scanned, and pruned when over the cap, on a background thread.
    pub fn new(dir: PathBuf, cap: u64, key: fn(&str) -> Option<&str>) -> Arc<Self> {
        let cache = Arc::new(Self { dir, key, cap, size: AtomicU64::new(0), pruning: Mutex::new(()) });
        let scan = cache.clone();
        std::thread::spawn(move || scan.prune(scan.cap, true));
        cache
    }

    /// Where the response of `url` is kept, `None` for a URL this cache doesn't hold.
    pub(crate) fn path(&self, url: &str) -> Option<PathBuf> {
        let stem = (self.key)(url).filter(|stem| is_plain(stem))?;
        Some(self.dir.join(format!("{stem}.{RECORD_EXTENSION}")))
    }

    /// The body stored at `path`, `None` when there is none or it is damaged. A damaged record is left for the
    /// refetched response to replace, so a concurrent write is never deleted by mistake.
    pub(crate) fn read(&self, path: &Path) -> Option<Bytes> {
        let file = File::open(path).ok()?;
        let len = file.metadata().ok()?.len();
        if !(HEADER_LEN as u64..=HEADER_LEN as u64 + MAX_BODY).contains(&len) {
            return None;
        }
        let mut record = Vec::with_capacity(len as usize);
        file.take(len).read_to_end(&mut record).ok()?;
        if !is_intact(&record) {
            log::warn!("Ignoring damaged cache record {}", path.display());
            return None;
        }
        Some(Bytes::from(record).slice(HEADER_LEN..))
    }

    /// Stores `body` at `path`, then prunes when the folder grew past the cap. Failures are logged and skipped.
    pub(crate) fn store(&self, path: &Path, body: &[u8]) {
        let replaced = fs::metadata(path).map_or(0, |m| m.len());
        if let Err(e) = self.write(path, body) {
            log::debug!("Not caching {}: {e}", path.display());
            return;
        }
        let added = ((HEADER_LEN + body.len()) as u64).saturating_sub(replaced);
        // Prune down to 7/8 of the cap, so the next few writes don't each scan the folder again. A write that crosses
        // the cap while a pass runs leaves it to that pass.
        if self.size.fetch_add(added, Ordering::Relaxed) + added > self.cap {
            self.prune(self.cap / 8 * 7, false);
        }
    }

    fn write(&self, path: &Path, body: &[u8]) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        // Dropping the temporary file on an error deletes it.
        let mut tmp = tempfile::Builder::new().prefix(TEMP_PREFIX).tempfile_in(&self.dir)?;
        tmp.write_all(MAGIC)?;
        tmp.write_all(&Sha256::digest(body))?;
        tmp.write_all(body)?;
        tmp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }

    /// Deletes the oldest records until the folder holds at most `target` bytes, and the temporary files of writes
    /// that never finished. Other files in the folder are left alone. With `wait` unset, a pass already running is
    /// left to do the work.
    fn prune(&self, target: u64, wait: bool) {
        let guard = match wait {
            true => self.pruning.lock().ok(),
            false => self.pruning.try_lock().ok(),
        };
        let Some(_guard) = guard else { return };
        let Ok(entries) = fs::read_dir(&self.dir) else {
            self.size.store(0, Ordering::Relaxed);
            return;
        };
        let now = SystemTime::now();
        let mut records = Vec::new();
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let path = entry.path();
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(TEMP_PREFIX) {
                if now.duration_since(modified).is_ok_and(|age| age > STALE_TEMP) {
                    let _ = fs::remove_file(&path);
                }
            } else if name.strip_suffix(RECORD_EXTENSION).and_then(|n| n.strip_suffix('.')).is_some_and(is_plain) {
                records.push((modified, meta.len(), path));
            }
        }
        let mut total: u64 = records.iter().map(|&(_, len, _)| len).sum();
        if total > target {
            // Records are written once, so the modification time is when they were stored: oldest first.
            records.sort_unstable_by_key(|&(modified, ..)| modified);
            for (_, len, path) in records {
                if total <= target {
                    break;
                }
                if fs::remove_file(&path).is_ok() {
                    total -= len;
                }
            }
        }
        self.size.store(total, Ordering::Relaxed);
    }
}

/// Whether `stem` can name a record: ASCII letters, digits and `-`, nothing that could leave the folder.
fn is_plain(stem: &str) -> bool {
    !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Whether `record` has the magic and its body matches the stored digest.
fn is_intact(record: &[u8]) -> bool {
    record.len() >= HEADER_LEN
        && record[..MAGIC.len()] == MAGIC[..]
        && Sha256::digest(&record[HEADER_LEN..])[..] == record[MAGIC.len()..HEADER_LEN]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(url: &str) -> Option<&str> {
        url.strip_prefix("https://cdn/")
    }

    fn cache(dir: &Path, cap: u64) -> DiskCache {
        DiskCache { dir: dir.to_path_buf(), key, cap, size: AtomicU64::new(0), pruning: Mutex::new(()) }
    }

    #[test]
    fn stored_bodies_read_back_and_damage_is_a_miss_until_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let cache = cache(dir.path(), u64::MAX);
        let path = cache.path("https://cdn/a").unwrap();
        assert_eq!(cache.read(&path), None);
        cache.store(&path, b"icon bytes");
        assert_eq!(cache.read(&path).as_deref(), Some(&b"icon bytes"[..]));

        let mut record = fs::read(&path).unwrap();
        *record.last_mut().unwrap() ^= 1;
        fs::write(&path, &record).unwrap();
        assert_eq!(cache.read(&path), None);
        cache.store(&path, b"icon bytes");
        assert_eq!(cache.read(&path).as_deref(), Some(&b"icon bytes"[..]));

        // Longer than any record could be: rejected before it is read.
        File::options().write(true).open(&path).unwrap().set_len(HEADER_LEN as u64 + MAX_BODY + 1).unwrap();
        assert_eq!(cache.read(&path), None);
    }

    #[test]
    fn only_plain_keys_map_to_files() {
        let cache = cache(Path::new("c"), u64::MAX);
        assert_eq!(cache.path("https://cdn/ab-12"), Some(Path::new("c").join("ab-12.camhttp")));
        for url in ["https://other/a", "https://cdn/", "https://cdn/../a", "https://cdn/a/b", "https://cdn/a.png"] {
            assert_eq!(cache.path(url), None, "{url}");
        }
    }

    #[test]
    fn pruning_deletes_the_oldest_records_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let record = (HEADER_LEN + 100) as u64;
        let cache = cache(dir.path(), 3 * record);
        let old = SystemTime::now() - Duration::from_secs(600);
        let age = |name: &str, offset: u64| {
            let file = File::options().write(true).open(dir.path().join(name)).unwrap();
            file.set_modified(old + Duration::from_secs(offset)).unwrap();
        };
        for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
            cache.store(&cache.path(&format!("https://cdn/{name}")).unwrap(), &[0; 100]);
            age(&format!("{name}.camhttp"), 1 + i as u64);
        }
        // Older than every record, but not the cache's.
        for foreign in ["addon.zip", "backup.bin", "a b.camhttp"] {
            fs::write(dir.path().join(foreign), [0; 1000]).unwrap();
            age(foreign, 0);
        }

        // The fourth record crosses the cap: the folder is pruned to 7/8 of it, oldest first.
        cache.store(&cache.path("https://cdn/d").unwrap(), &[0; 100]);
        let left = |name: &str| dir.path().join(name).exists();
        assert!(!left("a.camhttp") && !left("b.camhttp") && left("c.camhttp") && left("d.camhttp"));
        assert!(left("addon.zip") && left("backup.bin") && left("a b.camhttp"));
        assert_eq!(cache.size.load(Ordering::Relaxed), 2 * record);
    }
}
