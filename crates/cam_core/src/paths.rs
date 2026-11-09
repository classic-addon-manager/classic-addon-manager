//! The folders the app reads and writes, resolved once at startup.
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

/// Where the app keeps its files and looks for the game. The app resolves it once with [`Paths::from_env`], tests point
/// every folder into a temporary one.
#[derive(Clone, Debug)]
pub struct Paths {
    /// `config.toml`, `auth_session.json`, `managed_addons.json`, `app_state.json` and the logs.
    pub config: PathBuf,
    /// Downloaded addon archives and the Linux update failure report.
    pub cache: PathBuf,
    /// Where ArcheAge Classic installs are looked for: the drive root on Windows, the home directory elsewhere.
    pub install_root: PathBuf,
    /// The socket a running instance listens on.
    #[cfg(unix)]
    pub socket: PathBuf,
    /// `CAM_CONFIG_DIR` is set: the app runs against a test config, so it leaves the system alone and doesn't
    /// register itself as the handler of the sign-in links.
    pub sandboxed: bool,
}

impl Paths {
    /// The user's folders. `CAM_CONFIG_DIR` replaces the config folder and moves the cache and the socket into it, so a
    /// sandboxed run never touches the real ones. `CAM_CACHE_DIR` replaces the cache folder.
    pub fn from_env() -> Result<Self> {
        let env_dir = |name: &str| std::env::var_os(name).filter(|d| !d.is_empty()).map(PathBuf::from);
        let sandbox = env_dir("CAM_CONFIG_DIR");
        let config = match &sandbox {
            Some(dir) => dir.clone(),
            None => dirs::config_dir()
                .ok_or_else(|| anyhow!("could not get user config directory"))?
                .join("ClassicAddonManager"),
        };
        let cache = match (env_dir("CAM_CACHE_DIR"), &sandbox) {
            (Some(dir), _) => dir,
            (None, Some(dir)) => dir.join("cache"),
            (None, None) => dirs::cache_dir()
                .ok_or_else(|| anyhow!("could not get user cache directory"))?
                .join("ClassicAddonManager"),
        };
        #[cfg(windows)]
        let install_root = PathBuf::from("C:\\");
        #[cfg(not(windows))]
        let install_root = dirs::home_dir().ok_or_else(|| anyhow!("could not get user home directory"))?;
        #[cfg(unix)]
        let socket = match &sandbox {
            Some(dir) => dir.join("instance.sock"),
            None => PathBuf::from("/tmp/classic-addon-manager.sock"),
        };
        Ok(Self {
            config,
            cache,
            install_root,
            #[cfg(unix)]
            socket,
            sandboxed: sandbox.is_some(),
        })
    }

    /// The config folder, created when missing.
    pub fn data_dir(&self) -> Result<PathBuf> {
        ensure_dir(&self.config, "config")
    }

    /// The cache folder, created when missing.
    pub fn cache_dir(&self) -> Result<PathBuf> {
        ensure_dir(&self.cache, "cache")
    }
}

/// Creates `dir` and its parents. On unix a new folder is private to the user.
pub(super) fn mkdir_private(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(dir)
    }
}

/// Returns `dir`, creating it (private on unix) when missing.
pub(super) fn ensure_dir(dir: &Path, label: &str) -> Result<PathBuf> {
    match fs::metadata(dir) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            mkdir_private(dir).with_context(|| format!("create {label} directory {:?}", dir.display().to_string()))?;
            log::info!("Created {label} directory: {}", dir.display());
        }
        Err(e) => {
            return Err(
                anyhow::Error::new(e).context(format!("stat {label} directory {:?}", dir.display().to_string()))
            );
        }
    }
    Ok(dir.to_path_buf())
}
