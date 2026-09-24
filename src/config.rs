use crate::backends::Harness;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {

    /// Maximum length for sessions to live under `aw gc`
    #[serde(default = "default_max_session_idle_time")]
    pub max_session_idle_time: u64,

    /// Project directories to search when looking for repositories
    #[serde(default = "default_project_dirs")]
    pub project_dirs: Vec<PathBuf>,

    /// A static list of additional hard-coded repository paths
    pub extra_repos: Vec<PathBuf>,

    /// Agent harnesses to try, in order of preference. The first installed one is used
    #[serde(default = "default_harnesses")]
    pub harnesses: Vec<Harness>,

}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_session_idle_time: default_max_session_idle_time(),
            project_dirs: default_project_dirs(),
            extra_repos: Vec::new(),
            harnesses: default_harnesses(),
        }
    }
}

fn default_max_session_idle_time() -> u64 {
    72*3600
}

fn default_project_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("~/projects")]
}

fn default_harnesses() -> Vec<Harness> {
    vec![Harness::Claude, Harness::Opencode]
}

impl Config {
    /// Load the config from disk, falling back to defaults if it doesn't exist
    pub fn load(path: &Path) -> Self {
        let mut config: Config = match std::fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).expect("invalid config file"),
            Err(_) => Config::default(),
        };

        // Expand `~` and `$VARS` in all paths so the rest of the program can use them as-is
        for path in config.project_dirs.iter_mut().chain(config.extra_repos.iter_mut()) {
            *path = PathBuf::from(shellexpand::full(&path.to_string_lossy()).unwrap().as_ref());
        }
        config
    }
}
