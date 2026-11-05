use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Paths {
    pub config: PathBuf,
    pub state: PathBuf,
    pub socket: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self> {
        let home = env::var_os("HOME").context("HOME is not set")?;
        let home = PathBuf::from(home);
        let config_base = env_path("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let state_base = env_path("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));
        let runtime = env_path("XDG_RUNTIME_DIR")
            .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::geteuid() })));
        Ok(Self {
            config: config_base.join("sitbreak/config.toml"),
            state: state_base.join("sitbreak/state.json"),
            socket: runtime.join("sitbreak.sock"),
        })
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}
