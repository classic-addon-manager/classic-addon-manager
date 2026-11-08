//! Verifies a downloaded update against its published SHA-256 before it may be installed.
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};

pub(super) const ERR_UPDATE_VERIFICATION_FAILED: &str =
    "the update download could not be verified, please try again later or download the new version manually";

/// Removes `path`, a missing file counts as removed.
pub(super) fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Compares the file's SHA-256 with the published checksum. No checksum => allowed with a warning;
/// a mismatching file is deleted so it can never be installed.
pub(super) fn verify_update_checksum(path: &Path, expected: &str) -> Result<()> {
    let expected = expected.trim();
    if expected.is_empty() {
        log::warn!("No checksum available for this update, skipping verification");
        return Ok(());
    }

    let actual = match file_sha256(path) {
        Ok(a) => a,
        Err(e) => {
            log::error!("Error hashing downloaded update: {e:#}");
            remove_update_file(path);
            return Err(anyhow!(ERR_UPDATE_VERIFICATION_FAILED));
        }
    };

    if !actual.eq_ignore_ascii_case(expected) {
        log::error!("Update checksum mismatch: expected {}, got {actual}", expected.to_lowercase());
        remove_update_file(path);
        return Err(anyhow!(ERR_UPDATE_VERIFICATION_FAILED));
    }

    log::info!("Update checksum verified: {actual}");
    Ok(())
}

fn file_sha256(path: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn remove_update_file(path: &Path) {
    if let Err(e) = remove_if_exists(path) {
        log::error!("Error removing rejected update file: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_dir;
    use super::*;

    #[test]
    fn checksum_accepts_match_and_deletes_mismatch() {
        let d = test_dir("sum");
        let f = d.join("u.bin");
        fs::write(&f, b"hello").unwrap();
        let sum = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        verify_update_checksum(&f, sum).unwrap();
        verify_update_checksum(&f, &sum.to_uppercase()).unwrap();
        verify_update_checksum(&f, "").unwrap();
        let err = verify_update_checksum(&f, &"0".repeat(64)).unwrap_err();
        assert_eq!(err.to_string(), ERR_UPDATE_VERIFICATION_FAILED);
        assert!(!f.exists());
        fs::remove_dir_all(d).unwrap();
    }
}
