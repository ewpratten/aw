use super::Backend;
use std::path::Path;

pub struct Claude {
    pub remote_control: bool,
}

impl Backend for Claude {
    fn is_installed(&self) -> bool {
        super::responds_to_version("claude")
    }

    fn prepare(&self, work_dir: &Path) {
        // Pre-accept Claude's trust-this-folder dialog for the work dir
        let claude_config_path = directories::BaseDirs::new().unwrap().home_dir().join(".claude.json");
        if let Some(mut claude_config) = std::fs::read_to_string(&claude_config_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        {
            claude_config["projects"][work_dir.to_string_lossy().as_ref()]["hasTrustDialogAccepted"] = true.into();
            let temp_path = claude_config_path.with_extension("json.aw-tmp");
            std::fs::write(&temp_path, serde_json::to_string_pretty(&claude_config).unwrap()).unwrap();
            std::fs::rename(&temp_path, &claude_config_path).unwrap();
        }
    }

    fn extend_env(&self, session_env: &mut Vec<String>) {
        // Pass through API/gateway settings so Claude can reach custom endpoints (like the work AIGW)
        session_env.extend(
            std::env::vars()
                .filter(|(key, _)| {
                    key.starts_with("ANTHROPIC_")
                        || key.starts_with("CLAUDE_CODE_")
                        || ["DISABLE_ERROR_REPORTING", "ENABLE_TOOL_SEARCH"].contains(&key.as_str())
                })
                .map(|(key, value)| format!("{}={}", key, value)),
        );
    }

    fn command(&self, session: &str, prompt: Option<&str>) -> String {
        // Name the Remote Control session after the aw session so it's easy to find from other devices
        let flags = match self.remote_control {
            true => format!("--permission-mode auto --remote-control '{}'", session),
            false => "--permission-mode auto".to_string(),
        };
        match prompt {
            Some(prompt) => format!("claude {flags} {}", super::shell_quote(prompt)),
            None => format!("claude -c {flags} || claude {flags}"),
        }
    }
}
