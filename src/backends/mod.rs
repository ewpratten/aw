mod claude;
mod opencode;

use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

/// An agent harness that can be launched inside of an aw session
pub trait Backend {
    /// Check whether this harness is available on the system
    fn is_installed(&self) -> bool;

    /// Do any setup needed before the harness is launched in `work_dir`
    fn prepare(&self, _work_dir: &Path) {}

    /// Add any harness-specific variables to the tmux session environment
    fn extend_env(&self, _session_env: &mut Vec<String>) {}

    /// Shell command used to launch the harness
    fn command(&self) -> &'static str;
}

/// Harnesses that can be listed in the config
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Harness {
    Claude,
    Opencode,
}

impl Harness {
    /// Get the backend implementation for this harness
    pub fn backend(&self) -> Box<dyn Backend> {
        match self {
            Harness::Claude => Box::new(claude::Claude),
            Harness::Opencode => Box::new(opencode::OpenCode),
        }
    }
}

/// Check if a binary can be run with `--version`, discarding its output
fn responds_to_version(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
