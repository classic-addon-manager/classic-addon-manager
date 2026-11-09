//! Session token store. `auth_session.json` has the layout the Go client writes. The token in memory is the one every
//! API request sends.
use std::sync::RwLock;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paths::Paths;
use super::versioned_json::VersionedJson;

static TOKEN: RwLock<String> = RwLock::new(String::new());

pub fn get_token() -> String {
    TOKEN.read().unwrap_or_else(|p| p.into_inner()).clone()
}

pub fn set_token(t: &str) {
    *TOKEN.write().unwrap_or_else(|p| p.into_inner()) = t.to_string();
}

pub fn clear_token() {
    TOKEN.write().unwrap_or_else(|p| p.into_inner()).clear();
}

#[derive(Serialize, Deserialize)]
struct SessionFile {
    #[serde(default)]
    encoding: String,
    #[serde(default)]
    token: String,
    updated_at: DateTime<Utc>,
}

fn session_file(paths: &Paths) -> Result<VersionedJson<SessionFile>> {
    Ok(VersionedJson::new(paths.data_dir()?.join("auth_session.json"), 1, 0o600))
}

/// Reads the saved token. A session file that can't be used is moved aside, and the app starts signed out.
pub fn load_from_disk(paths: &Paths) {
    let file = match session_file(paths) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Auth: cannot resolve session file path: {e:#}");
            return;
        }
    };
    let sf = match file.load() {
        Ok(Some(sf)) => sf,
        Ok(None) => {
            log::info!("Auth: no session file, starting signed out");
            return;
        }
        Err(e) => {
            clear_token();
            log::error!("Auth: invalid session file: {e:#}");
            return;
        }
    };
    let plain = if sf.token.is_empty() {
        Err(anyhow!("empty token"))
    } else {
        unprotect_token(&sf.encoding, &sf.token).context("error decrypting token")
    };
    match plain {
        Ok(plain) => {
            set_token(&plain);
            log::info!("Auth: session loaded from disk");
        }
        Err(e) => {
            clear_token();
            // Not inside the log call: its arguments only run when the level is enabled.
            let e = file.quarantine(e);
            log::error!("Auth: invalid session file: {e:#}");
        }
    }
}

pub fn save_to_disk(paths: &Paths, token: &str) -> Result<()> {
    if token.is_empty() {
        return Err(anyhow!("token must not be empty"));
    }
    let (protected, encoding) = protect_token(token).context("error protecting token")?;
    let sf = SessionFile { encoding, token: protected, updated_at: Utc::now() };
    session_file(paths).context("error resolving session file path")?.save(&sf)?;
    set_token(token);
    log::info!("Auth: session saved to disk");
    Ok(())
}

pub fn delete_from_disk(paths: &Paths) -> Result<()> {
    let file = session_file(paths).context("error resolving session file path")?;
    match std::fs::remove_file(file.path()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(anyhow::Error::new(e).context("error deleting session file")),
    }
    clear_token();
    log::info!("Auth: session deleted from disk");
    Ok(())
}

#[cfg(not(windows))]
fn protect_token(plain: &str) -> Result<(String, String)> {
    Ok((plain.to_string(), "plain".to_string()))
}

#[cfg(not(windows))]
fn unprotect_token(encoding: &str, token: &str) -> Result<String> {
    if encoding == "plain" {
        return Ok(token.to_string());
    }
    Err(anyhow!("unsupported encoding on linux: {encoding}"))
}

#[cfg(windows)]
mod dpapi {
    use anyhow::{Context, Result, anyhow};
    use base64::Engine;

    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            p_data_in: *const DataBlob,
            sz_data_descr: *const u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *const core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            p_data_in: *const DataBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *const core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(h: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }

    fn run(data: &[u8], protect: bool) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(anyhow!("empty data"));
        }
        let input = DataBlob { cb_data: data.len() as u32, pb_data: data.as_ptr() as *mut u8 };
        let mut out = DataBlob { cb_data: 0, pb_data: core::ptr::null_mut() };
        // SAFETY: blobs point at valid memory for the call; the output buffer is copied then freed with LocalFree.
        unsafe {
            let ok = if protect {
                CryptProtectData(
                    &input,
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                    0,
                    &mut out,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    core::ptr::null_mut(),
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                    0,
                    &mut out,
                )
            };
            if ok == 0 {
                return Err(anyhow::Error::new(std::io::Error::last_os_error())
                    .context(format!("{} failed", if protect { "CryptProtectData" } else { "CryptUnprotectData" })));
            }
            let result = core::slice::from_raw_parts(out.pb_data, out.cb_data as usize).to_vec();
            LocalFree(out.pb_data as *mut _);
            Ok(result)
        }
    }

    pub fn protect(plain: &str) -> Result<(String, String)> {
        let enc = run(plain.as_bytes(), true).context("dpapi encrypt failed")?;
        Ok((base64::engine::general_purpose::STANDARD.encode(enc), "dpapi".to_string()))
    }

    pub fn unprotect(encoding: &str, token: &str) -> Result<String> {
        match encoding {
            "dpapi" => {
                let decoded =
                    base64::engine::general_purpose::STANDARD.decode(token).context("error decoding base64")?;
                let dec = run(&decoded, false).context("dpapi decrypt failed")?;
                Ok(String::from_utf8_lossy(&dec).into_owned())
            }
            "plain" => Ok(token.to_string()),
            other => Err(anyhow!("unknown encoding: {other}")),
        }
    }
}

#[cfg(windows)]
fn protect_token(plain: &str) -> Result<(String, String)> {
    dpapi::protect(plain)
}

#[cfg(windows)]
fn unprotect_token(encoding: &str, token: &str) -> Result<String> {
    dpapi::unprotect(encoding, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_go_session_file() {
        let file = VersionedJson::<SessionFile>::new("auth_session.json".into(), 1, 0o600);
        let sf = file
            .decode(br#"{"version":1,"encoding":"plain","token":"abc","updated_at":"2026-05-31T15:54:49.065809945Z"}"#)
            .unwrap();
        assert_eq!(unprotect_token(&sf.encoding, &sf.token).unwrap(), "abc");
    }

    #[test]
    fn rejects_unknown_encoding() {
        #[cfg(not(windows))]
        assert!(unprotect_token("dpapi", "x").is_err());
    }
}
