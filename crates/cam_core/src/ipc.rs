//! Single-instance IPC and `classicaddonmanager://auth?t=...` deeplink handling.
//! Unix: the socket in [`Paths::socket`]. Windows: loopback TCP.
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::path::PathBuf;

use super::paths::Paths;
use super::startup::DEEPLINK_SCHEME;

const MAX_LINE: u64 = 64 * 1024;

/// Returns true when `raw` is a `classicaddonmanager://auth` URL (scheme case-insensitive, host exactly `auth`).
pub fn is_auth_deeplink(raw: &str) -> bool {
    parse_auth_deeplink(raw).is_some()
}

/// For an auth deeplink: `Some(token)` where the token is the (possibly empty) `t` query value.
pub fn parse_auth_deeplink(raw: &str) -> Option<String> {
    if raw.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f) {
        return None;
    }
    let (scheme, rest) = raw.split_once(':')?;
    if !scheme.eq_ignore_ascii_case(DEEPLINK_SCHEME) {
        return None;
    }
    let rest = rest.strip_prefix("//")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if &rest[..end] != "auth" {
        return None;
    }
    let after = &rest[end..];
    let before_fragment = after.split('#').next().unwrap_or("");
    let query = before_fragment.split_once('?').map(|(_, q)| q).unwrap_or("");
    let token = form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "t")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();
    Some(token)
}

#[cfg(windows)]
const TCP_ADDR: &str = "127.0.0.1:47329";

/// This process as the running instance. [`Instance::start_server`] takes the sign-in links of later launches.
pub struct Instance {
    #[cfg(unix)]
    socket: PathBuf,
    /// Bound by [`claim_instance`] and handed to the server. `None` when binding failed for another reason than a
    /// taken port: the server tries again.
    #[cfg(windows)]
    listener: Option<std::net::TcpListener>,
}

/// Tries to reach an already running instance, forwarding `first_arg` when it is an auth deeplink. Returns `None` when
/// another instance answered (the caller should exit), otherwise this process is the instance.
#[cfg_attr(windows, allow(unused_variables))]
pub fn claim_instance(paths: &Paths, first_arg: Option<&str>) -> Option<Instance> {
    #[cfg(unix)]
    let (conn, instance) =
        (std::os::unix::net::UnixStream::connect(&paths.socket), Instance { socket: paths.socket.clone() });
    #[cfg(windows)]
    let (conn, instance) = {
        // Connecting to a loopback port nobody listens on takes ~2 s on Windows (the SYN is retried after each RST),
        // so claim the port instead: a successful bind means no other instance, and the listener is kept for
        // `start_server`. Only an occupied port is worth connecting to, which then answers immediately.
        match std::net::TcpListener::bind(TCP_ADDR) {
            Ok(listener) => return Some(Instance { listener: Some(listener) }),
            Err(e) if e.kind() != std::io::ErrorKind::AddrInUse => return Some(Instance { listener: None }),
            Err(_) => (std::net::TcpStream::connect(TCP_ADDR), Instance { listener: None }),
        }
    };

    match conn {
        Ok(mut c) => {
            if let Some(arg) = first_arg.filter(|a| is_auth_deeplink(a)) {
                let _ = writeln!(c, "{arg}");
            }
            None
        }
        Err(_) => Some(instance),
    }
}

impl Instance {
    /// Starts the IPC server on a background thread. `on_token` is called (from that thread) with the token of each
    /// valid auth deeplink; the UI should raise the window and emit the token into its auth flow.
    pub fn start_server(&mut self, on_token: impl Fn(String) + Send + Sync + 'static) {
        let on_token = std::sync::Arc::new(on_token);
        #[cfg(unix)]
        let path = self.socket.clone();
        #[cfg(windows)]
        let claimed = self.listener.take();
        std::thread::spawn(move || {
            #[cfg(unix)]
            {
                // Remove an existing socket (stale from a crash).
                let _ = std::fs::remove_file(&path);
                let listener = match std::os::unix::net::UnixListener::bind(&path) {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("Error starting Unix socket server: {e:#}");
                        return;
                    }
                };
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Err(e) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)) {
                        log::error!("Error setting socket permissions: {e:#}");
                    }
                }
                for conn in listener.incoming() {
                    match conn {
                        Ok(c) => {
                            let cb = on_token.clone();
                            std::thread::spawn(move || handle_connection(c, &*cb));
                        }
                        Err(e) => log::error!("Error accepting connection: {e:#}"),
                    }
                }
            }
            #[cfg(windows)]
            {
                let listener = match claimed.map_or_else(|| std::net::TcpListener::bind(TCP_ADDR), Ok) {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("Error starting pipe server: {e:#}");
                        return;
                    }
                };
                for conn in listener.incoming() {
                    match conn {
                        Ok(c) => {
                            let cb = on_token.clone();
                            std::thread::spawn(move || handle_connection(c, &*cb));
                        }
                        Err(e) => log::error!("Error accepting connection: {e:#}"),
                    }
                }
            }
        });
    }

    /// Removes the socket file (call on shutdown).
    pub fn cleanup(&self) {
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&self.socket);
        }
    }
}

fn handle_connection<C: Read>(conn: C, on_token: &dyn Fn(String)) {
    let mut reader = BufReader::new(conn.take(MAX_LINE));
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => {
            log::warn!("Received empty data from socket (no deeplink URL provided by client)");
            return;
        }
        Ok(_) => {}
        Err(e) => {
            log::error!("Error reading from socket: {e:#}");
            return;
        }
    }
    handle_deeplink(line.trim_end_matches(['\r', '\n']), on_token);
}

fn handle_deeplink(deeplink: &str, on_token: &dyn Fn(String)) {
    let Some(token) = parse_auth_deeplink(deeplink) else {
        log::warn!("Ignoring unrecognized deeplink");
        return;
    };
    if token.is_empty() {
        log::warn!("No token found in deeplink URL");
        return;
    }
    log::info!("Received authentication token");
    on_token(token);
}

/// Registers the URL scheme handler for the current user.
pub fn register_deeplink() {
    #[cfg(target_os = "linux")]
    register_deeplink_linux();
    #[cfg(windows)]
    register_deeplink_windows();
}

#[cfg(target_os = "linux")]
fn register_deeplink_linux() {
    let Some(home) = dirs::home_dir() else {
        log::error!("Failed to get home directory: no home directory");
        return;
    };
    let applications_dir = home.join(".local").join("share").join("applications");
    if let Err(e) = std::fs::create_dir_all(&applications_dir) {
        log::error!("Failed to create applications directory: {e:#}");
        return;
    }
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to get executable path: {e:#}");
            return;
        }
    };
    let desktop_file = applications_dir.join("classic-addon-manager.desktop");
    if desktop_file.exists() {
        log::info!("Deeplink handler already registered.");
        return;
    }
    let content = format!(
        "[Desktop Entry]\nVersion=1.0\nName=Classic Addon Manager\nComment=An addon manager for ArcheAge Classic\nExec={} %u\nTerminal=false\nType=Application\nIcon=classicaddonmanager\nCategories=Utility;\nStartupWMClass=classicaddonmanager\nMimeType=x-scheme-handler/classicaddonmanager;\n",
        exe.display()
    );
    if let Err(e) = std::fs::write(&desktop_file, content) {
        log::error!("Failed to write .desktop file: {e:#}");
        return;
    }
    log::info!("Desktop file created successfully at {}", desktop_file.display());
    log::info!("You may need to run: update-desktop-database ~/.local/share/applications");
}

#[cfg(windows)]
fn register_deeplink_windows() {
    let reg_path = format!(r"HKCU\Software\Classes\{DEEPLINK_SCHEME}");
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to get executable path: {e:#}");
            return;
        }
    };
    let command = format!("\"{}\" \"%1\"", exe.display());
    let steps: [Vec<String>; 3] = [
        vec!["add".into(), reg_path.clone(), "/ve".into(), "/d".into(), format!("URL:{DEEPLINK_SCHEME}"), "/f".into()],
        vec![
            "add".into(),
            reg_path.clone(),
            "/v".into(),
            "URL Protocol".into(),
            "/d".into(),
            String::new(),
            "/f".into(),
        ],
        vec!["add".into(), format!(r"{reg_path}\shell\open\command"), "/ve".into(), "/d".into(), command, "/f".into()],
    ];
    for args in steps {
        match crate::process::hidden_command("reg").args(&args).output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                log::error!("Failed to set registry value: {}", String::from_utf8_lossy(&o.stderr));
                return;
            }
            Err(e) => {
                log::error!("Failed to set registry value: {e:#}");
                return;
            }
        }
    }
    log::info!("Deeplink key created successfully");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeplink_detection_matches_go_cases() {
        let yes = [
            "classicaddonmanager://auth?t=abc",
            "ClassicAddonManager://auth?t=abc",
            "classicaddonmanager://auth/?t=abc",
            "classicaddonmanager://auth",
        ];
        let no = [
            "",
            "--check-updates",
            "-check-updates",
            r"C:\foo\bar.txt",
            "/tmp/file",
            "https://auth?t=abc",
            "classicaddonmanager://other?t=abc",
            "classicaddonmanager:auth?t=abc",
            "classicaddonmanager://auth?t=abc\n--check-updates",
            "classicaddonmanager://user@evil/auth",
        ];
        for s in yes {
            assert!(is_auth_deeplink(s), "{s}");
        }
        for s in no {
            assert!(!is_auth_deeplink(s), "{s}");
        }
    }

    #[test]
    fn token_extraction() {
        assert_eq!(parse_auth_deeplink("classicaddonmanager://auth?t=a%2Bb&x=1").unwrap(), "a+b");
        assert_eq!(parse_auth_deeplink("classicaddonmanager://auth").unwrap(), "");
        assert_eq!(parse_auth_deeplink("classicaddonmanager://auth?x=1#t=no").unwrap(), "");
    }

    #[cfg(unix)]
    #[test]
    fn server_roundtrip_forwards_deeplink() {
        let sb = crate::testutil::sandbox();
        let paths = sb.paths();
        let mut instance = claim_instance(paths, None).expect("no other instance runs in the sandbox");
        let (tx, rx) = std::sync::mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        instance.start_server(move |t| {
            let _ = tx.lock().unwrap().send(t);
        });
        let mut up = false;
        for _ in 0..100 {
            if paths.socket.exists() {
                up = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(up, "server did not start");
        assert!(claim_instance(paths, Some("classicaddonmanager://auth?t=abc")).is_none());
        assert_eq!(rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(), "abc");
        instance.cleanup();
    }

    #[test]
    fn connection_delivers_token_once() {
        let got = std::sync::Mutex::new(Vec::new());
        handle_connection(&b"classicaddonmanager://auth?t=tok\n"[..], &|t| got.lock().unwrap().push(t));
        handle_connection(&b"classicaddonmanager://auth\n"[..], &|t| got.lock().unwrap().push(t));
        handle_connection(&b"junk\n"[..], &|t| got.lock().unwrap().push(t));
        assert_eq!(*got.lock().unwrap(), vec!["tok".to_string()]);
    }
}
