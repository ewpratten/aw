use super::Backend;
use std::path::Path;

pub struct Claude;

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

    fn command(&self) -> &'static str {
        "claude -c --permission-mode auto || claude --permission-mode auto"
    }
}
